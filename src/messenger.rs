use self::message::{Message, MessageBase, Query, QueryResult};
use crate::{dht_node::DhtNode, utils::{self, LogErrExt}};
use futures::{future::{BoxFuture, FutureExt, RemoteHandle}, Future};
use log::*;
use message::QueryMethod;
use simple_error::{map_err_with, require_with, try_with, SimpleResult};
use std::{net::SocketAddr, ops::DerefMut, sync::Arc};
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
    pub async fn new(host_node: DhtNode, timeout_ms: u16, query_handler: QueryHandler) -> SimpleResult<Self> {
        let socket = map_err_with!(UdpSocket::bind(host_node.addr).await, "error binding host address")?;
        let queries_outbound = Mutex::new(Vec::with_capacity(20));
        let state =
            Arc::new(ServiceState { host_node, socket, queries_outbound, timeout_ms, queries_inbound: query_handler });
        let service = Service { state };
        let (job, _recv_handle) = service.clone().recv().remote_handle();

        let new = Messenger { _recv_handle, service };
        tokio::spawn(job);

        Ok(new)
    }

    // async fn handle_query(q: &Query, state: &Arc<State>) -> SimpleResult<()> {
    //     let base = Self::build_message_base(
    //         state,
    //         require_with!(q.received_from_addr, "no return address"),
    //         q.transaction_id.clone(),
    //     );
    //     let message = match q.method {
    //         QueryMethod::Ping => base.to_response(ReturnKind::Ok).to_message(),
    //         QueryMethod::FindNode(_) => todo!(),
    //         QueryMethod::GetPeers(_) => todo!(),
    //         QueryMethod::AnnouncePeer(_) => todo!(),
    //         QueryMethod::Put(_) => todo!(),
    //         QueryMethod::Get => todo!(),
    //     };
    //     Self::_send_message(&state, &message).await
    // }

    pub async fn send_message(&self, message: &Message) -> SimpleResult<()> {
        self.service.send_message(message).await
    }

    pub async fn query(&self, method: QueryMethod, to: SocketAddr) -> QueryResult {
        self.service.query(method, to).await
    }
}

//type QueryHandler = Option<fn(Query) -> Pin<Box<dyn Future<Output = QueryResult> + Send>>>;

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
