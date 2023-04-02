use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
    let base64 = base64::engine::general_purpose::STANDARD.encode(v);
    String::serialize(&base64, s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let base64 = String::deserialize(d)?;
    base64::engine::general_purpose::STANDARD
        .decode(base64.as_bytes())
        .map_err(serde::de::Error::custom)
}
