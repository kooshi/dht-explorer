use self::message::{kmsg::socket_addr_wrapper, Message};
use crate::utils;
use crate::{dht_node::DhtNode, routing_table::bucket::Bucket, u160::U160};
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use futures::{
    future::{AbortHandle, RemoteHandle},
    prelude::*,
};
use log::*;
use simple_error::bail;
use std::rc::Rc;
use std::{error::Error, net::SocketAddr, ops::DerefMut, sync::Arc};
use tokio::{net::UdpSocket, sync::Mutex};

pub(crate) mod message;

pub struct KrpcService {
    state: Arc<State>,
    _handle: RemoteHandle<()>,
}
struct State {
    socket: UdpSocket,
    routes: Mutex<Bucket>,
    outstanding_queries: Mutex<Vec<OutstandingQuery>>,
}
struct OutstandingQuery {
    transaction_id: String,
    timestamp: DateTime<Utc>,
    continue_with: Box<dyn Fn(Message) + Send + Sync>,
}

impl KrpcService {
    pub async fn new(host_node: DhtNode) -> Result<Self, Box<dyn Error>> {
        let socket = UdpSocket::bind(host_node.addr).await?;
        let routes = Mutex::new(Bucket::root(host_node, 8));
        let outstanding_queries = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(State {
            socket,
            routes,
            outstanding_queries,
        });
        let (job, _handle) =
            futures_util::future::FutureExt::remote_handle(KrpcService::recv(state.clone()));

        let new = KrpcService { state, _handle };
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
            message.kind(),
            message.transaction_id(),
            message.received_from_addr().unwrap()
        );
        debug!("Received: {:?}", message);
        {
            let mut queue = state.outstanding_queries.lock().await;
            if let Some(index) = queue
                .iter()
                .position(|q| q.transaction_id == message.transaction_id())
            {
                let q = queue.remove(index);
                debug!("Continue for [{}] executing", message.transaction_id());
                tokio::spawn(async move { (q.continue_with)(message) });
            }
        }
    }

    pub async fn send_message(&self, message: Message) {
        info!(
            "Sending {} [{}] to {}",
            message.kind(),
            message.transaction_id(),
            message.destination_addr().unwrap()
        );
        debug!("Sending: {:?}", message);

        let slice = &message.to_bytes();
        let addr = message.destination_addr().unwrap();
        self.state.socket.send_to(&slice, addr).await.unwrap();
    }

    pub async fn send_with_continue(
        &self,
        message: Message,
        continue_with: Box<dyn Fn(Message) + Sync + Send>,
    ) {
        {
            let mut queue = self.state.outstanding_queries.lock().await;
            queue.push(OutstandingQuery {
                transaction_id: message.transaction_id().to_string(),
                timestamp: chrono::offset::Utc::now(),
                continue_with,
            });
            debug!(
                "Query [{}] continuation added to queue",
                message.transaction_id()
            );
        }
        self.send_message(message).await;
    }
}
