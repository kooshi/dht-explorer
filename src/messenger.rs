use self::message::{KnownError, Message, Query, QueryResult};
use crate::utils::{self, LogErrExt};
use async_trait::async_trait;
use futures::future::{FutureExt, RemoteHandle};
use log::*;
use simple_error::{map_err_with, try_with, SimpleResult};
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{oneshot, Mutex, Semaphore};
use tokio::time;
use tokio::time::Duration;
pub(crate) mod message;
#[cfg(test)]
mod messenger_tests;
mod service;
use service::*;

pub struct Messenger {
    _recv_handle: RemoteHandle<()>,
    service:      Service,
}

impl Messenger {
    pub async fn new(
        bind_addr: SocketAddr, timeout_ms: u16, query_handler: Option<WrappedQueryHandler>, max_q: u8,
    ) -> SimpleResult<Self> {
        let socket = map_err_with!(UdpSocket::bind(bind_addr).await, "error binding host address")?;
        let queries_outbound = Mutex::new(Vec::with_capacity(20));
        let state = Arc::new(ServiceState {
            socket,
            queries_outbound,
            timeout_ms,
            queries_inbound: query_handler,
            packet_num: AtomicUsize::new(0),
            max_q: Semaphore::new(max_q.into()),
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

    pub async fn query_unbounded(&self, query: &Query) -> QueryResult {
        self.service.query_unbounded(query).await
    }
}

pub type WrappedQueryHandler = Arc<dyn QueryHandler + Send + Sync>;

#[async_trait]
pub trait QueryHandler {
    async fn handle_query(&self, query: Query) -> QueryResult;
    async fn handle_error(&self, tid: Vec<u8>, source_addr: SocketAddr, error: KnownError) -> message::Error;
}
