use self::message::{Message, MessageBase, Query, QueryResult};
use crate::{node_info::NodeInfo, utils::{self, LogErrExt}};
use futures::{future::{BoxFuture, FutureExt, RemoteHandle}, Future};
use log::*;
use message::QueryMethod;
use simple_error::{map_err_with, require_with, try_with, SimpleResult};
use std::{net::SocketAddr, ops::DerefMut, sync::{atomic::AtomicUsize, Arc}};
use tokio::{net::UdpSocket, sync::{oneshot, Mutex}, time, time::Duration};
pub(crate) mod message;
#[cfg(test)]
mod messenger_tests;
mod service;
use service::*;

pub struct Messenger {
    service:      Service,
    _recv_handle: RemoteHandle<()>,
}

impl Messenger {
    pub async fn new(host_node: NodeInfo, timeout_ms: u16, query_handler: QueryHandler) -> SimpleResult<Self> {
        let socket = map_err_with!(UdpSocket::bind(host_node.addr).await, "error binding host address")?;
        let queries_outbound = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(ServiceState {
            host_node,
            socket,
            queries_outbound,
            timeout_ms,
            queries_inbound: query_handler,
            packet_num: AtomicUsize::new(0),
        });
        let service = Service { state };
        let (job, _recv_handle) = service.clone().recv().remote_handle();

        let new = Messenger { _recv_handle, service };
        tokio::spawn(job);

        Ok(new)
    }

    pub async fn send_message(&self, message: &Message) -> SimpleResult<()> {
        self.service.send_message(message).await
    }

    pub async fn query(&self, method: QueryMethod, to: SocketAddr) -> QueryResult {
        self.service.query(method, to).await
    }
}

type QueryHandler = Option<Box<dyn AsyncHandler + Sync + Send>>;
pub trait AsyncHandler {
    fn call(&self, result_base: MessageBase, query: Query) -> BoxFuture<'static, QueryResult>;
}
impl<T, F> AsyncHandler for T
where
    T: Fn(MessageBase, Query) -> F,
    F: Future<Output = QueryResult> + Send + 'static,
{
    fn call(&self, result_base: MessageBase, query: Query) -> BoxFuture<'static, QueryResult> {
        Box::pin(self(result_base, query))
    }
}
