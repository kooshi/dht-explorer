extern crate base64;
use std::{ops::*, fmt};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct U160 {
    msbytes:u128,
    lsbytes:u32
}

impl U160 {
    pub fn new() -> Self {
        Self {
            msbytes: rand::random(),
            lsbytes: rand::random()
        }
    }
    pub fn empty() -> Self {
        Self {
            msbytes: 0,
            lsbytes: 0
        }
    }

    pub fn from_be_bytes(bytes:[u8;20]) -> Self {
        let mut msbytes = [0_u8;16];
        msbytes.copy_from_slice(&bytes[..16]);
        let mut lsbytes = [0_u8;4];
        lsbytes.copy_from_slice(&bytes[16..]);
        Self { msbytes: u128::from_be_bytes(msbytes), lsbytes: u32::from_be_bytes(lsbytes) }
    }

    pub fn distance(self, other:Self) -> Self {
        self ^ other
    }

    pub fn to_be_bytes(self) -> [u8;20] {
        let mut bytes = [0_u8;20];
        bytes[..16].copy_from_slice(&self.msbytes.to_be_bytes());
        bytes[16..].copy_from_slice(&self.lsbytes.to_be_bytes());
        bytes
    }

    pub fn get_bit(self, be_index:u8) -> bool {
        match be_index {
           x if x >= 160 => false,
           x if x >= 128 => (1_u32 << (31-(x-128)) ) & self.lsbytes != 0,
           x => 1_u128 << (127 - x) & self.msbytes != 0
        }
    }
}

impl BitXor for U160 {
    type Output = Self;
    // rhs is the "right-hand side" of the expression `a ^ b`
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self { msbytes: self.msbytes ^ rhs.msbytes, lsbytes: self.lsbytes ^ rhs.lsbytes }
    }
}

impl BitAnd for U160 {
    type Output = Self;
    // rhs is the "right-hand side" of the expression `a ^ b`
    fn bitand(self, rhs: Self) -> Self::Output {
        Self { msbytes: self.msbytes & rhs.msbytes, lsbytes: self.lsbytes & rhs.lsbytes }
    }
}

// impl Shl for U160 {
//     type Output;

//     fn shl(self, rhs: Self) -> Self::Output {
//         let carry = self.lsbytes & 0x800000_u32 >> 32;
//         Self { msbytes: self.msbytes << 1 | carry, lsbytes: self.lsbytes << 1}
//     }
// }

impl fmt::Display for U160 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}",base64::encode(self.to_be_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::U160;

    #[test]
    fn new() {
        let id = U160::new();
        println!("{0}",id.msbytes);
        println!("{0}",id.lsbytes);
        assert_ne!(id.msbytes, 0);
        assert_ne!(id.lsbytes, 0);
    }

    #[test]
    fn distance() {
        let one = U160::new();
        let distance = one.distance(one);
        assert_eq!(distance.lsbytes,0);
        assert_eq!(distance.msbytes,0);

        let two = U160::new();
        let distance2 = one.distance(two);
        assert_eq!(distance2,one^two);
    }

    #[test]
    fn ords() {
        let one = U160 { msbytes: 0, lsbytes: 1 };
        let two = U160{msbytes:0,lsbytes:2};
        let bigone = U160{msbytes:1,lsbytes:0};
        assert!(bigone > two);
        assert!(two > one);
        assert!(one > U160::empty());
    }

    #[test]
    fn display()  {
        let id = U160::new();
        println!("{:?}",id);
        println!("{}",id);
    }

    #[test]
    fn bytes() {
        let id = U160::new();
        let bytes = id.to_be_bytes();
        let id2 = U160::from_be_bytes(bytes);
        assert_eq!(id,id2);
    }

    #[test]
    fn bits() {
        let id = U160 {msbytes:0x80000000_00000000_00000000_00000000_u128, lsbytes: 0x80000000_u32 };
        assert!(id.get_bit(0));
        assert!(id.get_bit(128));
        
        let id = U160 {msbytes:0x00000000_00000000_00000000_00000001_u128, lsbytes: 0x00000001_u32 };
        assert!(id.get_bit(127));
        assert!(id.get_bit(159));

        let id = U160 {msbytes:0xFFFFFFFE_FFFFFFFF_00000000_00000001_u128, lsbytes: 0x00000001_u32 };
        assert!(!id.get_bit(31));
    }

}