use crate::u160::U160;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for U160 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_bytes(&self.to_be_bytes())
    }
}

impl<'de> Deserialize<'de> for U160 {
    fn deserialize<D>(deserializer: D) -> Result<U160, D::Error>
    where D: Deserializer<'de> {
        struct U160Visitor {}
        impl U160Visitor {
            fn make(v: &[u8]) -> U160 {
                let mut bytes = [0_u8; 20];
                bytes.copy_from_slice(v);
                U160::from_be_bytes(&bytes)
            }
        }
        impl<'de> serde::de::Visitor<'de> for U160Visitor {
            type Value = U160;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("20 big endian bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where E: serde::de::Error {
                if v.len() > 20 {
                    return Err(Error::invalid_length(v.len(), &self));
                }
                if v.len() < 20 {
                    //some stupid nodes return a 2 byte id????
                    Ok(U160Visitor::make(&vec![vec![0; 20 - v.len()], v.to_vec()].concat()))
                } else {
                    Ok(U160Visitor::make(v))
                }
            }
        }
        deserializer.deserialize_bytes(U160Visitor {})
    }
}

pub fn int_from_bool<S>(value: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    if value.is_some() && value.unwrap() { serializer.serialize_i32(1) } else { serializer.serialize_i32(0) }
}

pub fn bool_from_int<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where D: Deserializer<'de> {
    match u8::deserialize(deserializer)? {
        0 => Ok(Some(false)),
        1 => Ok(Some(true)),
        other => Err(Error::invalid_value(Unexpected::Unsigned(other as u64), &"zero or one")),
    }
}
