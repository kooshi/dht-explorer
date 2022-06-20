use super::kmsg::KMessage;
use super::*;
use kmsg;
use serde::de;
use serde::ser;
use serde::{Deserialize, Deserializer, Serialize};
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_kmsg().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Message, D::Error>
    where
        D: Deserializer<'de>,
    {
        Message::from_kmsg(KMessage::deserialize(deserializer)?).map_err(|e|de::Error::custom(e))
    }
}
