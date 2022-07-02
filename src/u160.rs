extern crate hex;
use crate::utils::{Ipv4AddrExt, Ipv6AddrExt};
use crc32c::crc32c;
use std::fmt;
use std::net::IpAddr;
use std::ops::*;
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U160 {
    msbytes: u128,
    lsbytes: u32,
}

#[allow(clippy::unusual_byte_groupings)]
impl U160 {
    //bep42
    pub fn from_ip(addr: &IpAddr) -> Self {
        let r = rand::random::<u8>() & 0x07_u8;
        Self::from_ip_with_r(addr, r)
    }

    fn from_ip_with_r(addr: &IpAddr, r: u8) -> Self {
        let pfx = match addr {
            IpAddr::V4(ip) => crc32c(&((ip.to_u32() & 0x030f3fff_u32) | ((r as u32) << 29)).to_be_bytes()),
            IpAddr::V6(ip) => crc32c(&((ip.ms_u64() & 0x0103070f_1f3f7fff_u64) | ((r as u64) << 61)).to_be_bytes()),
        };
        let msbytes = (rand::random::<u128>() & 0x000007ff_ffffffff_ffffffff_ffffffff_u128)
            | (((pfx & 0xfffff800_u32) as u128) << 96);
        let lsbytes = (rand::random::<u32>() & 0xfffffff8_u32) | ((r & 0x07_u8) as u32);
        Self::new(msbytes, lsbytes)
    }

    pub fn validate(&self, addr: &IpAddr) -> bool {
        let r = self.lsbytes.to_be_bytes()[3] & 0x07_u8;
        let test = Self::from_ip_with_r(addr, r);
        let filter = Self::new(0xfffff800_00000000_00000000_00000000_u128, 0x00000007_u32);
        (*self & filter) == (test & filter)
    }

    pub fn new(msbytes: u128, lsbytes: u32) -> Self {
        Self { msbytes, lsbytes }
    }

    pub fn rand() -> Self {
        Self { msbytes: rand::random(), lsbytes: rand::random() }
    }

    pub fn empty() -> Self {
        Self { msbytes: 0, lsbytes: 0 }
    }

    pub fn distance(self, other: Self) -> Self {
        self ^ other
    }

    pub fn from_hex(hex: &str) -> Self {
        let mut bytes = [0_u8; 20];
        hex::decode_to_slice(hex, &mut bytes).expect("error getting id from hex");
        Self::from_be_bytes(&bytes)
    }

    pub fn from_be_bytes(bytes: &[u8; 20]) -> Self {
        let mut msbytes = [0_u8; 16];
        msbytes.copy_from_slice(&bytes[..16]);
        let mut lsbytes = [0_u8; 4];
        lsbytes.copy_from_slice(&bytes[16..]);
        Self::new(u128::from_be_bytes(msbytes), u32::from_be_bytes(lsbytes))
    }

    pub fn to_be_bytes(self) -> [u8; 20] {
        let mut bytes = [0_u8; 20];
        bytes[..16].copy_from_slice(&self.msbytes.to_be_bytes());
        bytes[16..].copy_from_slice(&self.lsbytes.to_be_bytes());
        bytes
    }

    pub fn get_bit(self, be_index: u8) -> bool {
        match be_index {
            x if x >= 160 => false,
            x if x >= 128 => (1_u32 << (31 - (x - 128))) & self.lsbytes != 0,
            x => 1_u128 << (127 - x) & self.msbytes != 0,
        }
    }
}

impl BitXor for U160 {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self::new(self.msbytes ^ rhs.msbytes, self.lsbytes ^ rhs.lsbytes)
    }
}

impl BitAnd for U160 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::new(self.msbytes & rhs.msbytes, self.lsbytes & rhs.lsbytes)
    }
}

impl BitOr for U160 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::new(self.msbytes | rhs.msbytes, self.lsbytes | rhs.lsbytes)
    }
}

impl Not for U160 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::new(!self.msbytes, !self.lsbytes)
    }
}

impl Shr<u8> for U160 {
    type Output = Self;

    fn shr(self, rhs: u8) -> Self::Output {
        let msshift = rhs.min(127);
        let msremain = rhs - msshift;
        let msbytes = if msremain > 0 { 0 } else { self.msbytes >> msshift };
        let overflow = ((self.msbytes << (127 - msshift)) >> msremain >> (127 - 32)) as u32;

        let lsshift = rhs.min(31);
        let lsremain = rhs - lsshift;
        let lsbytes = if lsremain > 0 { 0 } else { self.lsbytes >> lsshift };
        let lsbytes = lsbytes | overflow;

        Self::new(msbytes, lsbytes)
    }
}

impl Shl<u8> for U160 {
    type Output = Self;

    fn shl(self, rhs: u8) -> Self::Output {
        let lsshift = rhs.min(31);
        let lsremain = rhs - lsshift;
        let lsbytes = if lsremain > 0 { 0 } else { self.lsbytes << lsshift };
        let overflow =
            if lsshift == 0 || lsremain > 127 { 0 } else { ((self.lsbytes >> (32 - lsshift)) as u128) << lsremain };

        let msshift = rhs.min(127);
        let msremain = rhs - msshift;
        let msbytes = if msremain > 0 { 0 } else { self.msbytes << msshift };
        let msbytes = msbytes | overflow as u128;

        Self::new(msbytes, lsbytes)
    }
}

impl fmt::Display for U160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64::encode(self.to_be_bytes()))
    }
}

impl fmt::Debug for U160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", base64::encode(self.to_be_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::U160;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn new() {
        let id = U160::rand();
        println!("{0}", id.msbytes);
        println!("{0}", id.lsbytes);
        assert_ne!(id.msbytes, 0);
        assert_ne!(id.lsbytes, 0);
    }

    #[test]
    fn distance() {
        let one = U160::rand();
        let distance = one.distance(one);
        assert_eq!(distance.lsbytes, 0);
        assert_eq!(distance.msbytes, 0);

        let two = U160::rand();
        let distance2 = one.distance(two);
        assert_eq!(distance2, one ^ two);
    }

    #[test]
    fn ords() {
        let one = U160::new(0, 1);
        let two = U160::new(0, 2);
        let bigone = U160::new(1, 0);
        assert!(bigone > two);
        assert!(two > one);
        assert!(one > U160::empty());
    }

    #[test]
    fn display() {
        let id = U160::rand();
        println!("{:?}", id);
        println!("{}", id);
    }

    #[test]
    fn bytes() {
        let id = U160::rand();
        let bytes = id.to_be_bytes();
        let id2 = U160::from_be_bytes(&bytes);
        assert_eq!(id, id2);
    }

    #[allow(clippy::unusual_byte_groupings)]
    #[test]
    fn bits() {
        let id = U160::new(0x80000000_00000000_00000000_00000000_u128, 0x80000000_u32);
        assert!(id.get_bit(0));
        assert!(id.get_bit(128));

        let id = U160::new(0x00000000_00000000_00000000_00000001_u128, 0x00000001_u32);
        assert!(id.get_bit(127));
        assert!(id.get_bit(159));

        let id = U160::new(0xfffffffe_ffffffff_00000000_00000001_u128, 0x00000001_u32);
        assert!(!id.get_bit(31));
    }

    #[test]
    fn hex() {
        let id1 = U160::from_hex("5b19e3ca091fd1105b5ad3e7f1b8bd61e80ccd0c");
        let id2 = U160::from_hex("5b19e3ca091fd1105b5ad3e7f1b8bd61e80ccd1c");
        let dis = U160::from_hex("0000000000000000000000000000000000000010");
        assert_eq!(dis, id1 ^ id2);

        let id1_copy = U160::from_hex(&id1.to_string());
        assert_eq!(id1, id1_copy);
    }

    #[test]
    fn shifts() {
        let one = U160::from_hex("0000000000000000000000000000000000000001");
        let two = U160::from_hex("0000000000000000000000000000000000000002");
        assert_eq!(two >> 1, one);
        assert_eq!(one << 1, two);

        let one = U160::from_hex("4000000000000000100000000000000080000000");
        let two = U160::from_hex("8000000000000000200000000000000100000001");
        assert_eq!(two >> 1, one);
        assert_eq!(one << 1 | U160::new(0, 1), two);

        let one = U160::from_hex("5b19e3ca091fd1105b5ad3e7f1b8bd61e80ccd1c");
        let two = U160::from_hex("05b19e3ca091fd1105b5ad3e7f1b8bd61e80ccd1");
        let thr = U160::from_hex("b19e3ca091fd1105b5ad3e7f1b8bd61e80ccd1c0");
        assert_eq!(one >> 4, two);
        assert_eq!(one >> 0, one);
        assert_eq!(one << 4, thr);
        assert_eq!(one << 0, one);

        let one = U160::from_hex("fb19e3ca091fd1105b5ad3e7f1b8bd61e80ccd1c");
        let two = U160::from_hex("00000000000000000000000000000000fb19e3ca");
        let thr = U160::from_hex("fb19e3ca00000000000000000000000000000000");
        assert_eq!(one >> 128, two);
        assert_eq!(two << 128, thr);

        assert_eq!(U160::rand() >> 160, U160::empty());
        assert_eq!(U160::rand() << 160, U160::empty());
    }

    #[test]
    fn bep42() {
        let addr = IpAddr::from_str("2001:db8:85a3:0:0:8a2e:370:7334").unwrap();
        let test = U160::from_ip(&addr);
        println!("{}", test);
        assert!(test.validate(&addr));

        let addr = IpAddr::from_str("124.31.75.21").unwrap();
        let test = U160::from_ip(&addr);
        println!("{}", test);
        assert!(test.validate(&addr));

        assert!(
            U160::from_hex("5fbfbff10c5d6a4ec8a88e4c6ab4c28b95eee401")
                .validate(&IpAddr::from_str("124.31.75.21").unwrap())
        );
        assert!(
            U160::from_hex("5a3ce9c14e7a08645677bbd1cfe7d8f956d53256")
                .validate(&IpAddr::from_str("21.75.31.124").unwrap())
        );
        assert!(
            U160::from_hex("a5d43220bc8f112a3d426c84764f8c2a1150e616")
                .validate(&IpAddr::from_str("65.23.51.170").unwrap())
        );
        assert!(
            U160::from_hex("1b0321dd1bb1fe518101ceef99462b947a01ff41")
                .validate(&IpAddr::from_str("84.124.73.14").unwrap())
        );
        assert!(
            U160::from_hex("e56f6cbf5b7c4be0237986d5243b87aa6d51305a")
                .validate(&IpAddr::from_str("43.213.53.83").unwrap())
        );
        assert!(!U160::rand().validate(&IpAddr::from_str("43.213.53.83").unwrap()))
    }
}
