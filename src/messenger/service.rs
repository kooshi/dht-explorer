use super::*;
use crate::messenger::message::IMessageBase;
use simple_error::bail;
use std::sync::atomic::Ordering;
use tokio::sync::Semaphore;
pub struct ServiceState {
    pub socket:           UdpSocket,
    pub queries_outbound: Mutex<Vec<OutstandingQuery>>,
    pub timeout_ms:       u16,
    pub queries_inbound:  Option<WrappedQueryHandler>,
    pub packet_num:       AtomicUsize,
    pub max_q:            Semaphore,
}

#[derive(Clone)]
pub struct Service {
    pub state: Arc<ServiceState>,
}

pub struct OutstandingQuery {
    transaction_id:   Vec<u8>,
    destination_addr: SocketAddr,
    return_value:     oneshot::Sender<QueryResult>,
    timestamp_millis: i64,
}

impl Service {
    pub async fn recv(self) {
        let mut buffer = Box::new([0_u8; u16::MAX as usize]);
        loop {
            let result = self.state.socket.readable().await;
            if result.is_err() {
                error!("Waiting for UDP Socket: {:?}", result);
            }
            match self.state.socket.try_recv_from(buffer.deref_mut()) {
                Ok((n, from)) => {
                    let packet = self.state.packet_num.fetch_add(1, Ordering::Relaxed);
                    let slice = &buffer[..n];
                    trace!("[P:{packet}] <<<<< {}", utils::safe_string_from_slice(slice));
                    debug!("[P:{packet}] <<<<< {}", base64::encode(slice));
                    match Message::receive(slice, from) {
                        Ok(message) => {
                            tokio::spawn(self.clone().handle_received(packet, message));
                        }
                        Err((Some(raw), err)) => {
                            error!("[P:{packet}] Deserializing Message: {err:?} (CULPRIT: {})", base64::encode(slice));
                            if let Some(handler) = &self.state.queries_inbound {
                                self.send_message(
                                    &handler.handle_error(raw.transaction_id, from, err).await.into_message(),
                                )
                                .await
                                .log();
                            }
                        }
                        Err((None, err)) =>
                            error!("[P:{packet}] Deserializing Message: {err:?} (CULPRIT: {})", base64::encode(slice)),
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    error!("Reading from Socket: {}", e);
                }
            }
        }
    }

    async fn handle_received(self, packet: usize, message: Message) {
        debug!("[P:{packet}] <<<<< {message:?}");
        match message {
            Message::Query(q) =>
                if let Some(handler) = &self.state.queries_inbound {
                    info!("[P:{packet}][T:{} Q<<<<]{q}", hex::encode(&q.base().transaction_id));
                    self.send_message(&handler.handle_query(q).await.into()).await.log()
                } else {
                    warn!(
                        "[P:{packet}][T:{} DROPD] READ ONLY SERVICE! DROPPING: {q}",
                        hex::encode(&q.base().transaction_id)
                    );
                },
            Message::Response(r) => self.return_result(packet, Ok(r)).await,
            Message::Error(e) => self.return_result(packet, Err(e)).await,
        }
    }

    async fn return_result(&self, packet: usize, result: QueryResult) {
        let log_str = match &result {
            Ok(r) => format!("{r}"),
            Err(e) => format!("{e}"),
        };
        let id = &result.base().transaction_id;
        if let Some(waiting) = self.remove_from_queue(id, result.base().origin.addr()).await {
            info!(
                "[P:{packet}][T:{} {:03}ms]{log_str}",
                hex::encode(id),
                chrono::Local::now().timestamp_millis() - waiting.timestamp_millis
            );
            waiting.return_value.send(result).ok();
        } else {
            warn!("[P:{packet}][T:{} DROPD] NO ONE WAITING! DROPPING: {log_str}", hex::encode(id),);
        }
    }

    pub async fn query(&self, query: &Query) -> QueryResult {
        let _permit = self.state.max_q.acquire().await;
        self.query_unbounded(query).await
    }

    pub async fn query_unbounded(&self, query: &Query) -> QueryResult {
        // if cfg!(debug_assertions) && !cfg!(test) {
        //     time::sleep(Duration::from_millis(1000)).await;
        // }
        let (return_tx, return_rx) = oneshot::channel();
        {
            let mut queue = self.state.queries_outbound.lock().await;
            queue.push(OutstandingQuery {
                transaction_id:   query.transaction_id.clone(),
                destination_addr: query.origin.addr(),
                return_value:     return_tx,
                timestamp_millis: chrono::Local::now().timestamp_millis(),
            });
        }
        trace!("Query [T:{}] added to outstanding", hex::encode(&query.transaction_id));
        let message = query.clone().into_message();
        self.send_message(&message).await.map_err(|e| message.base().clone().into_error_generic(&e.to_string()))?;

        let sleep = time::sleep(Duration::from_millis(self.state.timeout_ms.into()));
        tokio::select! {
            m = return_rx => {
                m.map_or_else(
                |e|Result::Err(message.base().clone().into_error_generic(&e.to_string())),|r|r) }
            _ = sleep => {
                self.remove_from_queue(&query.transaction_id, message.origin.addr()).await;
                warn!("Query [T:{}] timed out", hex::encode(&query.transaction_id));
                Result::Err(message.base().clone().into_error_generic("Timeout"))
            }
        }
    }

    async fn remove_from_queue(&self, id: &Vec<u8>, queried_addr: SocketAddr) -> Option<OutstandingQuery> {
        trace!("Removing [T:{}] from queue", hex::encode(id));
        let mut queue = self.state.queries_outbound.lock().await;
        queue
            .iter()
            .position(|q| q.transaction_id == *id)
            .map(|i| queue.remove(i))
            .or_else(|| queue.iter().position(|q| q.destination_addr == queried_addr).map(|i| queue.remove(i)))
    }

    pub async fn send_message(&self, message: &Message) -> SimpleResult<()> {
        if !message.base().read_only && self.state.queries_inbound.is_none() {
            bail!("read only false but no query handler available")
        }

        let packet = self.state.packet_num.fetch_add(1, Ordering::Relaxed);
        info!("[P:{packet}]{message}");
        debug!("[P:{packet}] >>>>> {message:?}");
        let slice = message.to_bytes()?;
        trace!("[P:{packet}] >>>>> {}", utils::safe_string_from_slice(&slice));
        let addr = match message.destination {
            message::Receiver::Node(n) => n.addr,
            message::Receiver::Addr(a) => a,
            message::Receiver::Me => bail!("You probably didn't mean to message yourself like this"),
        };
        try_with!(self.state.socket.send_to(&slice, addr).await, "Send failed");
        Ok(())
    }
}
