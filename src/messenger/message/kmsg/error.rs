use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Error(pub u16, pub String);

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut s = serializer.serialize_seq(Some(2))?;
        s.serialize_element(&self.0)?;
        s.serialize_element(&self.1)?;
        s.end()
    }
}
impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> Result<Error, D::Error>
    where D: Deserializer<'de> {
        struct ErrorVisitor {}
        impl<'de> serde::de::Visitor<'de> for ErrorVisitor {
            type Value = Error;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expected error code followed by message")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where A: serde::de::SeqAccess<'de> {
                let zero = seq.next_element()?.unwrap_or_default();
                let one = seq.next_element()?.unwrap_or_default();
                Ok(Error(zero, one))
            }
        }
        deserializer.deserialize_seq(ErrorVisitor {})
    }
}
