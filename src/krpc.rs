use self::message::{Message, MessageBase, Query, ResponseKind};
use crate::{dht_node::DhtNode, routing_table::bucket::Bucket, utils::{self, LogErrExt}};
use futures::future::{FutureExt, RemoteHandle};
use log::*;
use message::QueryMethod;
use simple_error::{map_err_with, require_with, try_with, SimpleResult};
use std::{error::Error, net::SocketAddr, ops::DerefMut, sync::Arc};
use tokio::{net::UdpSocket, sync::{oneshot, Mutex}, time, time::Duration};
#[cfg(test)]
mod krpc_tests;
pub(crate) mod message;

pub struct KrpcService {
    state:        Arc<State>,
    _recv_handle: RemoteHandle<()>,
}
struct State {
    socket:              UdpSocket,
    outstanding_queries: Mutex<Vec<OutstandingQuery>>,
    host_node:           DhtNode,
    timeout_ms:          u16,
    read_only:           bool,
}
struct OutstandingQuery {
    transaction_id: String,
    return_value:   oneshot::Sender<Message>,
}

impl KrpcService {
    pub async fn new(host_node: DhtNode, timeout_ms: u16, read_only: bool) -> Result<Self, Box<dyn Error>> {
        let socket = UdpSocket::bind(host_node.addr).await?;
        let outstanding_queries = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(State { host_node, socket, outstanding_queries, timeout_ms, read_only });

        let (job, _recv_handle) = FutureExt::remote_handle(KrpcService::recv(state.clone()));

        let new = KrpcService { _recv_handle, state };
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
                        tokio::spawn(Self::handle_received(message, state_clone));
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
            message.transaction_id,
            message.received_from_addr.map_or("<unknown>".to_string(), |a| a.to_string())
        );
        debug!("Received: {:?}", message);

        match message {
            Message::Query(q) => Self::handle_query(&q, &state).await.log(),
            _ => {
                let id = &message.transaction_id;
                if let Some(q) = Self::remove_from_queue(&state, id).await {
                    debug!("Returning value for [{}]", id);
                    q.return_value.send(message).ok();
                }
            }
        }
    }

    async fn handle_query(q: &Query, state: &Arc<State>) -> SimpleResult<()> {
        if state.read_only {
            debug!("Query received by read only service. Dropped.");
            return Ok(());
        }

        let base = Self::build_message_base(
            state,
            require_with!(q.received_from_addr, "no return address"),
            q.transaction_id.clone(),
        );
        let message = match q.method {
            QueryMethod::Ping => base.to_response(ResponseKind::Ok).to_message(),
            QueryMethod::FindNode(_) => todo!(),
            QueryMethod::GetPeers(_) => todo!(),
            QueryMethod::AnnouncePeer(_) => todo!(),
            QueryMethod::Put(_) => todo!(),
            QueryMethod::Get => todo!(),
        };
        Self::_send_message(&state, &message).await
    }

    pub async fn send_message(&self, message: &Message) -> SimpleResult<()> {
        Self::_send_message(&self.state, message).await
    }

    async fn _send_message(state: &Arc<State>, message: &Message) -> SimpleResult<()> {
        info!(
            "Sending {} [{}] to {}",
            message,
            message.transaction_id,
            message.destination_addr.map_or("<unknown>".to_string(), |a| a.to_string())
        );
        debug!("Sending: {:?}", message);

        let slice = message.to_bytes()?;
        let addr = require_with!(message.destination_addr, "No send address");
        try_with!(state.socket.send_to(&slice, addr).await, "Send failed");
        Ok(())
    }

    fn build_message_base(state: &Arc<State>, to: SocketAddr, transaction_id: String) -> MessageBase {
        MessageBase::builder()
            .sender_id(state.host_node.id)
            .transaction_id(transaction_id)
            .destination_addr(to)
            .read_only(state.read_only)
            .build()
    }

    pub async fn query(&self, method: QueryMethod, to: SocketAddr) -> SimpleResult<Message> {
        let msg = Self::build_message_base(&self.state, to, rand::random::<u32>().to_string()).to_query(method);
        Self::_query(&self.state, msg).await
    }

    async fn _query(state: &Arc<State>, query: Query) -> SimpleResult<Message> {
        let (return_tx, return_rx) = oneshot::channel();
        {
            let mut queue = state.outstanding_queries.lock().await;
            queue.push(OutstandingQuery { transaction_id: query.transaction_id.clone(), return_value: return_tx });
        }
        debug!("Query [{}] added to outstanding", query.transaction_id);
        let message = Message::Query(query);
        Self::_send_message(state, &message).await?;

        let sleep = time::sleep(Duration::from_millis(state.timeout_ms.into()));
        tokio::select! {
            m = return_rx => { map_err_with!(m, "channel recv fail") }
            _ = sleep => {
                Self::remove_from_queue(&state, &message.transaction_id).await;
                info!("Query [{}] timed out", message.transaction_id);
                Ok(Message::Error( message.base().clone().to_error_generic("Timeout")))
            }
        }
    }

    async fn remove_from_queue(state: &Arc<State>, id: &str) -> Option<OutstandingQuery> {
        trace!("Removing [{}] from queue", id);
        let mut queue = state.outstanding_queries.lock().await;
        queue.iter().position(|q| q.transaction_id == id).map(|i| queue.remove(i))
    }
}
