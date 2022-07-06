use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha20;
use crc32c::crc32c;
use std::net::IpAddr;
use std::time::SystemTime;

const TOKEN_VALID_SECONDS: u64 = 120;

pub struct TokenGenerator {
    key: [u8; 32],
}

impl TokenGenerator {
    pub fn new() -> Self {
        Self { key: rand::random() }
    }

    pub fn generate(&self, ip: IpAddr) -> [u8; 20] {
        let rand_bytes: [u8; 8] = rand::random();
        let mut timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_be_bytes();
        let mut ipbytes = match ip {
            IpAddr::V4(ip) => ip.octets().to_vec(),
            IpAddr::V6(ip) => ip.octets().to_vec(),
        };
        ipbytes.extend_from_slice(&timestamp);
        ipbytes.extend_from_slice(&rand_bytes);
        let iphash = crc32c(&ipbytes).to_be_bytes();
        let mut nonce = [0; 12];
        nonce[..4].copy_from_slice(&iphash);
        nonce[4..].copy_from_slice(&rand_bytes);
        let mut cipher = ChaCha20::new(&self.key.into(), &nonce.into());
        cipher.apply_keystream(&mut timestamp);
        let mut token = [0_u8; 20];
        token[..12].copy_from_slice(&nonce);
        token[12..].copy_from_slice(&timestamp);
        token
    }

    pub fn validate(&self, token: &[u8], ip: IpAddr) -> bool {
        if token.len() != 20 {
            return false;
        }
        let mut nonce = [0; 12];
        nonce.copy_from_slice(&token[..12]);
        let mut timestamp = [0; 8];
        timestamp.copy_from_slice(&token[12..]);
        let mut cipher = ChaCha20::new(&self.key.into(), &nonce.into());
        cipher.apply_keystream(&mut timestamp);
        let token_timestamp = u64::from_be_bytes(timestamp);
        let mut ipbytes = match ip {
            IpAddr::V4(ip) => ip.octets().to_vec(),
            IpAddr::V6(ip) => ip.octets().to_vec(),
        };
        let mut rand_bytes = [0; 8];
        rand_bytes.copy_from_slice(&nonce[4..]);
        ipbytes.extend_from_slice(&timestamp);
        ipbytes.extend_from_slice(&rand_bytes);
        let mut iphash_test = [0; 4];
        iphash_test.copy_from_slice(&nonce[..4]);
        let iphash = crc32c(&ipbytes).to_be_bytes();
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        (iphash == iphash_test)
            && (token_timestamp <= timestamp)
            && ((token_timestamp + TOKEN_VALID_SECONDS) > timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::TokenGenerator;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn gen() {
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        let g = TokenGenerator::new();
        let t = g.generate(ip);
        assert!(!g.validate(
            &[103, 189, 190, 188, 134, 237, 102, 193, 221, 182, 236, 244, 250, 150, 229, 211, 218, 235, 192, 176],
            ip
        ));
        let ip2 = IpAddr::from_str("127.0.0.2").unwrap();
        assert!(!g.validate(&t, ip2));
        assert!(g.validate(&t, ip));
    }
}
