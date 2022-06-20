
use bt_bencode::value;
use serde::{de::{Error,Unexpected},Serialize,Deserialize,Deserializer,Serializer};
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


pub fn int_from_bool<S>(value: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    if value.is_some() && value.unwrap() {
        serializer.serialize_i32(1)
    } else {
        serializer.serialize_i32(0)
    }
}

pub fn bool_from_int<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(Some(false)),
        1 => Ok(Some(true)),
        other => Err(Error::invalid_value(
            Unexpected::Unsigned(other as u64),
            &"zero or one",
        )),
    }
}