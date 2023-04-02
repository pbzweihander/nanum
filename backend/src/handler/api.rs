use aws_sdk_s3::primitives::ByteStream;
use axum::{
    body::{Bytes, StreamBody},
    extract::{Path, State},
    http::StatusCode,
    routing, Json, Router,
};
use serde::Deserialize;

use crate::{s3, types::Metadata};

use super::{auth::User, AppState};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(get_health))
        .route(
            "/metadata/:id",
            routing::get(get_metadata).post(post_metadata),
        )
        .route("/file/:id", routing::get(get_file).post(post_file))
}

async fn get_health() -> &'static str {
    "OK"
}

async fn get_metadata(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Metadata>, (StatusCode, &'static str)> {
    let metadata = s3::get_metadata(&state.s3_client, &id)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to get metadata from S3");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get metadata from S3",
            )
        })?;
    Ok(Json(metadata))
}

#[derive(Deserialize)]
struct PostMetadataReq {
    #[serde(with = "crate::utils::base64")]
    pub salt: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename_nonce: Vec<u8>,
    #[serde(with = "crate::utils::base64")]
    pub filename: Vec<u8>,
    pub size: usize,
}

async fn post_metadata(
    user: User,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<PostMetadataReq>,
) -> Result<(), (StatusCode, &'static str)> {
    let metadata = Metadata {
        creator_email: user.primary_email,
        salt: req.salt,
        nonce: req.nonce,
        filename_nonce: req.filename_nonce,
        filename: req.filename,
        size: req.size,
    };
    s3::upload_metadata(&state.s3_client, &id, &metadata)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to upload metadata to S3");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to upload metadata to S3",
            )
        })?;
    Ok(())
}

async fn get_file(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<StreamBody<ByteStream>, (StatusCode, &'static str)> {
    let file = s3::get_file(&state.s3_client, &id).await.map_err(|error| {
        tracing::error!(%error, "failed to get file from S3");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to get file from S3",
        )
    })?;
    Ok(StreamBody::new(file))
}

async fn post_file(
    _user: User,
    Path(id): Path<String>,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<(), (StatusCode, &'static str)> {
    s3::upload_file(&state.s3_client, &id, body.to_vec())
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to upload file to S3");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to upload file to S3",
            )
        })?;
    Ok(())
}
