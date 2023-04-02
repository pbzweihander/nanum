use anyhow::Result;
use aws_sdk_s3::{primitives::ByteStream, Client};
use nanum_core::types::Metadata;

use crate::config::CONFIG;

fn key_file(id: &str, seq: usize) -> String {
    format!("file/{id}.{seq}")
}

fn key_metadata(id: &str) -> String {
    format!("metadata/{id}.json")
}

async fn get_object(s3_client: &Client, key: &str) -> Result<ByteStream> {
    let resp = s3_client
        .get_object()
        .bucket(&CONFIG.s3_bucket_name)
        .key(key)
        .send()
        .await?;
    Ok(resp.body)
}

pub async fn get_file(s3_client: &Client, id: &str, seq: usize) -> Result<ByteStream> {
    get_object(s3_client, &key_file(id, seq)).await
}

pub async fn get_metadata(s3_client: &Client, id: &str) -> Result<Metadata> {
    let resp = get_object(s3_client, &key_metadata(id)).await?;
    let body = resp.collect().await?.to_vec();
    let metadata = serde_json::from_slice(&body)?;
    Ok(metadata)
}

pub async fn upload_metadata(s3_client: &Client, id: &str, metadata: &Metadata) -> Result<()> {
    s3_client
        .put_object()
        .bucket(&CONFIG.s3_bucket_name)
        .key(key_metadata(id))
        .body(serde_json::to_vec(metadata)?.into())
        .send()
        .await?;
    Ok(())
}

pub async fn upload_file(s3_client: &Client, id: &str, seq: usize, data: Vec<u8>) -> Result<()> {
    s3_client
        .put_object()
        .bucket(&CONFIG.s3_bucket_name)
        .key(key_file(id, seq))
        .body(data.into())
        .send()
        .await?;
    Ok(())
}
