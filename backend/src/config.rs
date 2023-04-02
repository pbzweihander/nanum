use anyhow::{Context, Result};
use jsonwebtoken::{DecodingKey, EncodingKey};
use once_cell::sync::Lazy;
use serde::Deserialize;
use url::Url;

fn default_listen_addr() -> String {
    "0.0.0.0:3000".to_string()
}

fn deserialize_allowed_emails<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(s.split(',').map(str::to_string).collect())
}

fn default_public_url() -> Url {
    "http://localhost:3000/".parse().unwrap()
}

fn deserialize_jwt_secret<'de, D>(d: D) -> Result<(EncodingKey, DecodingKey), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok((
        EncodingKey::from_secret(s.as_bytes()),
        DecodingKey::from_secret(s.as_bytes()),
    ))
}

fn default_random_uri_length() -> usize {
    8
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    #[serde(deserialize_with = "deserialize_allowed_emails")]
    pub allowed_emails: Vec<String>,

    pub github_client_id: String,
    pub github_client_secret: String,

    #[serde(default = "default_public_url")]
    pub public_url: Url,

    #[serde(deserialize_with = "deserialize_jwt_secret")]
    pub jwt_secret: (EncodingKey, DecodingKey),

    pub s3_bucket_name: String,

    #[serde(default = "default_random_uri_length")]
    pub random_uri_length: usize,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    envy::from_env()
        .context("failed to parse config from environment variables")
        .unwrap()
});
