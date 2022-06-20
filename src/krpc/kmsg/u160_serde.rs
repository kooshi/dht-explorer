
use serde::{Serialize,Deserialize,Deserializer};
use crate::u160::U160;

impl Serialize for U160 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.to_be_bytes())
    }
}
impl<'de> Deserialize<'de> for U160 {
    fn deserialize<D>(deserializer: D) -> Result<U160, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct U160Visitor {}
        impl<'de> serde::de::Visitor<'de> for U160Visitor {
            type Value = U160;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expected 20 big endian bytes")
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
                let mut bytes = [0_u8;20];
                bytes.copy_from_slice(v);
                Ok(U160::from_be_bytes(&bytes))
            }
        }
        deserializer.deserialize_bytes(U160Visitor {})
    }
}