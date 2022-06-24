use self::message::{Message, Query, QueryResult};
use crate::utils::{self, LogErrExt};
use async_trait::async_trait;
use futures::{future::{BoxFuture, FutureExt, RemoteHandle}, Future};
use log::*;
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
    pub async fn new(
        bind_addr: SocketAddr, timeout_ms: u16, query_handler: Option<WrappedQueryHandler>,
    ) -> SimpleResult<Self> {
        let socket = map_err_with!(UdpSocket::bind(bind_addr).await, "error binding host address")?;
        let queries_outbound = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(ServiceState {
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

    pub async fn query(&self, query: &Query) -> QueryResult {
        self.service.query(query).await
    }
}

pub type WrappedQueryHandler = Arc<dyn QueryHandler + Send + Sync>;

#[async_trait]
pub trait QueryHandler {
    async fn handle_query(&self, query: Query) -> QueryResult;
}
