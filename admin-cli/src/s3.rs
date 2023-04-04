use anyhow::Result;
use aws_sdk_s3::{
    error::SdkError, operation::get_object::GetObjectError, primitives::ByteStream, Client,
};
use futures_util::{TryFutureExt, TryStreamExt};
use nanum_core::types::Metadata;

pub async fn list_metadatas(client: &Client, bucket: &str) -> Result<Vec<(String, Metadata)>> {
    client
        .list_objects_v2()
        .bucket(bucket)
        .prefix("metadata/")
        .into_paginator()
        .send()
        .err_into::<anyhow::Error>()
        .map_ok(|output| {
            futures_util::stream::iter(
                output
                    .contents
                    .unwrap_or_default()
                    .into_iter()
                    .map(Result::<_, anyhow::Error>::Ok),
            )
        })
        .try_flatten()
        .try_filter_map(|content| async move {
            if let Some(key) = content.key() {
                if let Some(name) = key.strip_prefix("metadata/") {
                    if let Some(name) = name.strip_suffix(".json") {
                        if let Some(resp) = get_object(client, bucket, key).await? {
                            let body = resp.collect().await?.into_bytes();
                            return Ok(serde_json::from_slice::<Metadata>(&body)
                                .ok()
                                .map(|metadata| (name.to_string(), metadata)));
                        }
                    }
                }
            }
            Ok(None)
        })
        .try_collect()
        .await
}

async fn get_object(s3_client: &Client, bucket: &str, key: &str) -> Result<Option<ByteStream>> {
    let resp = s3_client.get_object().bucket(bucket).key(key).send().await;
    match resp {
        Ok(resp) => Ok(Some(resp.body)),
        Err(error) => {
            if let SdkError::ServiceError(error) = error {
                if let GetObjectError::NoSuchKey(_) = error.err() {
                    Ok(None)
                } else {
                    Err(SdkError::ServiceError(error).into())
                }
            } else {
                Err(error.into())
            }
        }
    }
}

pub async fn delete_metadata(client: &Client, bucket: &str, id: &str) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(format!("metadata/{id}.json"))
        .send()
        .await?;
    Ok(())
}

pub async fn delete_file(client: &Client, bucket: &str, id: &str) -> Result<()> {
    client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(format!("file/{}.", id))
        .into_paginator()
        .send()
        .err_into::<anyhow::Error>()
        .map_ok(|output| {
            futures_util::stream::iter(
                output
                    .contents
                    .unwrap_or_default()
                    .into_iter()
                    .map(Result::<_, anyhow::Error>::Ok),
            )
        })
        .try_flatten()
        .try_filter_map(|content| async move { Ok(content.key().map(str::to_string)) })
        .and_then(|key| {
            client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .err_into()
        })
        .map_ok(|_| ())
        .try_collect::<()>()
        .await
}
