use std::{net::SocketAddr, error::Error, sync::Arc, ops::DerefMut};
use simple_error::bail;
use tokio::{net::UdpSocket, sync::Mutex};
use crate::{routing_table::bucket::Bucket, u160::U160, dht_node::DhtNode};
use futures::{prelude::*, future::{AbortHandle, RemoteHandle}};
use log::*;
use crate::utils;
use self::message::{kmsg::socket_addr_wrapper, Message};

pub(crate) mod message;

pub struct KrpcService{
    socket:Arc<UdpSocket>,
    routes:Mutex<Bucket>,
    //todo hashmap outstanding queries
    _handle:RemoteHandle<()>
}

impl KrpcService {
    pub async fn new(host_node:DhtNode) -> Result<Self, Box<dyn Error>> {
        let socket = Arc::new(UdpSocket::bind(host_node.addr).await?);
        let routes = Mutex::new(Bucket::root(host_node, 8));
        let (job, _handle) = futures_util::future::FutureExt::remote_handle(KrpcService::recv(socket.clone()));
        let new = KrpcService { socket, routes, _handle };
        tokio::spawn(job);

        Ok(new)
    }

    async fn recv(socket:Arc<UdpSocket>) {
        let mut buffer = Box::new([0_u8;u16::MAX as usize]);
        loop {
            let result = socket.readable().await;
            if result.is_err() {
                error!("Waiting for UDP Socket: {:?}", result);
            }
            // Try to recv data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match socket.try_recv_from(buffer.deref_mut()) {
                Ok((n, from)) => {
                    let slice = &buffer[..n];
                    debug!("UDP DATAGRAM: {}", utils::safe_string_from_slice(slice));
                    debug!("      BASE64: {}", base64::encode(slice));
                    let message = Message::receive(slice, from);
                    if let Ok(message) = message {
                        tokio::spawn(async move { KrpcService::handle_received(message).await });
                    } else {
                        error!("Deserializing Message: {:?}", message);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                Err(e) => {
                    error!("Reading from Socket: {}",e);
                    panic!("{}",e)
                }
            }
        }
    }

    async fn handle_received(message:Message) {
        info!("Received {} [{}] from {}", message.kind(), message.transaction_id(), message.received_from_addr().unwrap());
        debug!("Received: {:?}", message);
    }

    pub async fn send_message(&self, message:Message) {
        info!("Sending {} [{}] to {}",message.kind(),message.transaction_id(), message.destination_addr().unwrap());
        debug!("Sending: {:?}", message);

        let slice = &message.to_bytes();
        let addr = message.destination_addr().unwrap();
        self.socket.send_to(&slice, addr).await.unwrap();
    }
}