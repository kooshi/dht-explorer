use super::*;
use std::sync::atomic::Ordering;
pub struct ServiceState {
    pub socket:           UdpSocket,
    pub queries_outbound: Mutex<Vec<OutstandingQuery>>,
    pub host_node:        NodeInfo,
    pub timeout_ms:       u16,
    pub queries_inbound:  QueryHandler,
    pub packet_num:       AtomicUsize,
}

#[derive(Clone)]
pub struct Service {
    pub state: Arc<ServiceState>,
}

pub struct OutstandingQuery {
    transaction_id: String,
    return_value:   oneshot::Sender<QueryResult>,
}

impl Service {
    pub async fn recv(self) {
        let mut buffer = Box::new([0_u8; u16::MAX as usize]);
        loop {
            let result = self.state.socket.readable().await;
            if result.is_err() {
                error!("Waiting for UDP Socket: {:?}", result);
            }
            // Try to recv data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match self.state.socket.try_recv_from(buffer.deref_mut()) {
                Ok((n, from)) => {
                    let packet = self.state.packet_num.fetch_add(1, Ordering::Relaxed);
                    let slice = &buffer[..n];
                    trace!(" {} : Received: {}", packet, utils::safe_string_from_slice(slice));
                    debug!(" {} : Received: {}", packet, base64::encode(slice));
                    let message = Message::receive(slice, from);
                    if let Ok(message) = message {
                        tokio::spawn(self.clone().handle_received(packet, message));
                    } else {
                        error!(" {} :Deserializing Message: {:?}", packet, message);
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

    async fn handle_received(self, packet: usize, message: Message) {
        debug!(" {} : Received: {:?}", packet, message);
        info!(
            " {} : Received {} [{}] from {}",
            packet,
            message,
            message.transaction_id,
            message.received_from_addr.map_or("<unknown>".to_string(), |a| a.to_string())
        );

        let id = message.transaction_id.to_owned();
        match message {
            Message::Query(q) =>
                if let Some(handler) = &self.state.queries_inbound {
                    let base = self.build_message_base(q.received_from_addr.unwrap(), q.transaction_id.clone());
                    self.send_message(&handler.call(base, q).await.into()).await.log()
                } else {
                    debug!("Query received by read only service. Dropped.");
                },
            Message::Response(r) => self.return_result(&id, Ok(r)).await,
            Message::Error(e) => self.return_result(&id, Err(e)).await,
        }
    }

    async fn return_result(&self, id: &str, result: QueryResult) {
        if let Some(waiting) = self.remove_from_queue(id).await {
            debug!("Returning value for [{}]", id);
            waiting.return_value.send(result).ok();
        }
    }

    pub async fn query(&self, method: QueryMethod, to: SocketAddr) -> QueryResult {
        let query = self.build_message_base(to, rand::random::<u32>().to_string()).to_query(method);
        let (return_tx, return_rx) = oneshot::channel();
        {
            let mut queue = self.state.queries_outbound.lock().await;
            queue.push(OutstandingQuery { transaction_id: query.transaction_id.clone(), return_value: return_tx });
        }
        debug!("Query [{}] added to outstanding", query.transaction_id);
        let message = Message::Query(query);
        self.send_message(&message).await.map_err(|e| {
            self.build_message_base(self.state.host_node.addr, "".to_owned()).to_error_generic(&e.to_string())
        })?;

        let sleep = time::sleep(Duration::from_millis(self.state.timeout_ms.into()));
        tokio::select! {
            m = return_rx => {
                m.map_or_else(
                |e|Result::Err(self.build_message_base(self.state.host_node.addr, "".to_owned()).to_error_generic(&e.to_string())),|r|r) }
            _ = sleep => {
                self.remove_from_queue(&message.transaction_id).await;
                info!("Query [{}] timed out", message.transaction_id);
                Result::Err(message.base().clone().to_error_generic("Timeout"))
            }
        }
    }

    async fn remove_from_queue(&self, id: &str) -> Option<OutstandingQuery> {
        trace!("Removing [{}] from queue", id);
        let mut queue = self.state.queries_outbound.lock().await;
        queue.iter().position(|q| q.transaction_id == id).map(|i| queue.remove(i))
    }

    pub async fn send_message(&self, message: &Message) -> SimpleResult<()> {
        let packet = self.state.packet_num.fetch_add(1, Ordering::Relaxed);
        info!(
            " {} : Sending {} [{}] to {}",
            packet,
            message,
            message.transaction_id,
            message.destination_addr.map_or("<unknown>".to_string(), |a| a.to_string())
        );
        debug!(" {} : Sending: {:?}", packet, message);
        let slice = message.to_bytes()?;
        let addr = require_with!(message.destination_addr, "No send address");
        trace!(" {} : Sending: {}", packet, utils::safe_string_from_slice(&slice));
        try_with!(self.state.socket.send_to(&slice, addr).await, "Send failed");
        Ok(())
    }

    pub fn build_message_base(&self, to: SocketAddr, transaction_id: String) -> MessageBase {
        MessageBase::builder()
            .sender_id(self.state.host_node.id)
            .transaction_id(transaction_id)
            .destination_addr(to)
            .read_only(self.state.queries_inbound.is_none())
            .build()
    }
}
