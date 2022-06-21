use self::message::{IMessage, Message, Query};
use crate::utils;
use crate::{dht_node::DhtNode, routing_table::bucket::Bucket};
use chrono::{DateTime, Utc};
use futures::future::{FutureExt, RemoteHandle};
use log::*;
use std::{error::Error, ops::DerefMut, sync::Arc};
use tokio::sync::oneshot;
use tokio::{net::UdpSocket, sync::Mutex};
use tokio::{select, time, time::Duration};

pub(crate) mod message;

pub struct KrpcService {
    state: Arc<State>,
    recv_handle: RemoteHandle<()>,
    timeout_ms: u16,
}
struct State {
    socket: UdpSocket,
    routes: Mutex<Bucket>,
    outstanding_queries: Mutex<Vec<OutstandingQuery>>,
}
struct OutstandingQuery {
    transaction_id: String,
    timestamp: DateTime<Utc>,
    return_value: oneshot::Sender<Message>,
}

impl KrpcService {
    pub async fn new(host_node: DhtNode, timeout_ms: u16) -> Result<Self, Box<dyn Error>> {
        let socket = UdpSocket::bind(host_node.addr).await?;
        let routes = Mutex::new(Bucket::root(host_node, 8));
        let outstanding_queries = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(State {
            socket,
            routes,
            outstanding_queries,
        });

        let (job, recv_handle) = FutureExt::remote_handle(KrpcService::recv(state.clone()));

        let new = KrpcService {
            state,
            recv_handle,
            timeout_ms,
        };
        tokio::spawn(job);

        Ok(new)
    }

    async fn recv(state: Arc<State>) {
        let mut buffer = Box::new([0_u8; u16::MAX as usize]);
        loop {
            let result = state.socket.readable().await;
            if result.is_err() {
                error!("Waiting for UDP Socket: {:?}", result);
            }
            // Try to recv data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match state.socket.try_recv_from(buffer.deref_mut()) {
                Ok((n, from)) => {
                    let slice = &buffer[..n];
                    debug!("UDP DATAGRAM: {}", utils::safe_string_from_slice(slice));
                    debug!("      BASE64: {}", base64::encode(slice));
                    let message = Message::receive(slice, from);
                    if let Ok(message) = message {
                        let state_clone = state.clone();
                        tokio::spawn(async move {
                            KrpcService::handle_received(message, state_clone).await
                        });
                    } else {
                        error!("Deserializing Message: {:?}", message);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    error!("Reading from Socket: {}", e);
                    panic!("{}", e)
                }
            }
        }
    }

    async fn handle_received(message: Message, state: Arc<State>) {
        info!(
            "Received {} [{}] from {}",
            message,
            message.data().transaction_id(),
            message.data().received_from_addr().unwrap()
        );
        debug!("Received: {:?}", message);
        {
            let mut queue = state.outstanding_queries.lock().await;
            if let Some(index) = queue
                .iter()
                .position(|q| q.transaction_id == message.data().transaction_id())
            {
                let q = queue.remove(index);
                debug!("Returning value for [{}]", message.data().transaction_id());
                q.return_value.send(message);
            }
        }
    }

    pub async fn send_message(&self, message: Message) {
        info!(
            "Sending {} [{}] to {}",
            message,
            message.data().transaction_id(),
            message.data().destination_addr().unwrap()
        );
        debug!("Sending: {:?}", message);

        let slice = &message.to_bytes();
        let addr = message.data().destination_addr().unwrap();
        self.state.socket.send_to(&slice, addr).await.unwrap();
    }

    pub async fn query(&self, query: Query) -> Result<Message, oneshot::error::RecvError> {
        let (return_tx, return_rx) = oneshot::channel();
        {
            let mut queue = self.state.outstanding_queries.lock().await;
            queue.push(OutstandingQuery {
                transaction_id: query.transaction_id().to_string(),
                timestamp: chrono::offset::Utc::now(),
                return_value: return_tx,
            });
        }
        debug!("Query [{}] added to outstanding", query.transaction_id());
        let message = query.to_message();
        let clone_data = message.data().clone();
        self.send_message(message).await;

        let sleep = time::sleep(Duration::from_millis(self.timeout_ms.into()));
        tokio::select! {
            m = return_rx => {m}
            _ = sleep => {
                self.remove_from_queue(clone_data.transaction_id()).await;
                info!("Query [{}] timed out", clone_data.transaction_id());
                Ok(Message::Error(message::Error::new(201,"Timeout".to_string(), clone_data)))
            }
        }
    }

    async fn remove_from_queue(&self, id: &str) {
        trace!("Removing [{}] from queue", id);
        let mut queue = self.state.outstanding_queries.lock().await;
        if let Some(index) = queue.iter().position(|q| q.transaction_id == id) {
            queue.remove(index);
        }
    }
}
