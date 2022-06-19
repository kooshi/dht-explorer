
use serde::{Serialize, Deserialize, ser::SerializeSeq, Deserializer};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Error (pub u16, pub String);

impl Error {
    // ErrorTimeout is returned when the query times out.
    pub fn error_timeout() -> Self { Self (201,"Query timeout".to_string()) }
    
    // ErrorMethodUnknown is returned when the query verb is unknown.
    pub fn error_method_unknown() -> Self  { Self (  204,   "Method Unknown".to_string())}

    // ErrorProtocolError is returned when a malformed incoming packet.
    pub fn error_protocol_error() -> Self  { Self ( 203,  "Protocol Error, such as a malformed packet, invalid arguments, or bad token".to_string())}

    // ErrorBadToken is returned when an unknown incoming token.
    pub fn error_bad_token() -> Self  { Self::error_protocol_error() }

    // ErrorVTooLong is returned when a V is too long (>999).
    pub fn error_vtoo_long() -> Self  { Self ( 205,  "message (v field) too big.".to_string())}

    // ErrorVTooShort is returned when a V is too small (<1).
    pub fn error_vtoo_short() -> Self  { Self ( 205,  "message (v field) too small.".to_string())}

    // ErrorInvalidSig is returned when a signature is invalid.
    pub fn error_invalid_sig() -> Self  { Self ( 206,  "invalid signature".to_string())}

    // ErrorNoK is returned when k is missing.
    pub fn error_no_k() -> Self  { Self ( 206,  "invalid k".to_string())}

    // ErrorSaltTooLong is returned when a salt is too long (>64).
    pub fn error_salt_too_long() -> Self  { Self ( 207,  "salt (salt field) too big.".to_string())}

    // ErrorCasMismatch is returned when the cas mismatch (put).
    pub fn error_cas_mismatch() -> Self  { Self ( 301,  "the CAS hash mismatched, re-read value and try again.".to_string())}

    // ErrorSeqLessThanCurrent is returned when the incoming seq is less than current (get/put).
    pub fn error_seq_less_than_current() -> Self  { Self ( 302,  "sequence number less than current.".to_string())}

    // ErrorInternalIssue is returned when an internal error occurs.
    pub fn error_internal_issue() -> Self  { Self ( 501,  "an internal error prevented the operation to succeed.".to_string())}

    // ErrorInsecureNodeID is returned when an incoming query with an insecure id is detected.
    pub fn error_insecure_node_id() -> Self  { Self ( 305,  "Invalid node id.".to_string())}
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut s = serializer.serialize_seq(Some(2))?;
            s.serialize_element(&self.0);
            s.serialize_element(&self.1);
            s.end()
    }
}
impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> Result<Error, D::Error>
    where
        D: Deserializer<'de> {
            struct ErrorVisitor {}
            impl<'de> serde::de::Visitor<'de> for ErrorVisitor {   
                type Value = Error;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str(&format!("expected error code followed by message"))
                }
                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>, {
                    let zero = seq.next_element()?.unwrap_or_default();
                    let one = seq.next_element()?.unwrap_or_default();
                    Ok(Error(zero,one))
                }
            }
            deserializer.deserialize_seq(ErrorVisitor {})
    }
}