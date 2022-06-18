use crate::u160::U160;
use std::net::{self, SocketAddr};

#[derive(Debug, Copy, Clone)]
pub struct DhtNode {
    pub id:U160,
    pub addr:SocketAddr
}
impl DhtNode {
    // pub fn new() -> Self {
    //     let (foo,addr) = net::UdpSocket::bind("127.0.0.1:3400").expect("couldn't bind to address").recv_from([0_u8;10].as_mut());
    //     DhtNode { id:U160::new() }
    // }
    pub fn distance(&self, other:&Self) -> U160 {
        self.id.distance(other.id)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn new() {
        println!("Hello Test");
    }

}