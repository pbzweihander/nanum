use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub creator_email: String,
    #[serde(with = "crate::utils::base64")]
    pub salt: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename_nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename: Vec<u8>,
    pub size: usize,
    pub block_size: usize,
}

#[derive(Serialize, Deserialize)]
pub struct MetadataCreationReq {
    #[serde(with = "crate::utils::base64")]
    pub salt: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename_nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename: Vec<u8>,
    pub size: usize,
    pub block_size: usize,
}

impl MetadataCreationReq {
    pub fn into_metadata(self, creator_email: String) -> Metadata {
        let Self {
            salt,
            nonce,
            filename_nonce,
            filename,
            size,
            block_size,
        } = self;
        Metadata {
            creator_email,
            salt,
            nonce,
            filename_nonce,
            filename,
            size,
            block_size,
        }
    }
}
