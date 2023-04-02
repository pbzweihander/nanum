use aws_sdk_s3::primitives::ByteStream;
use axum::{
    body::{Bytes, StreamBody},
    extract::{Path, State},
    http::StatusCode,
    routing, Json, Router,
};
use nanum_core::types::{Metadata, MetadataCreationReq};
use serde::{Deserialize, Serialize};

use crate::{config::CONFIG, s3};

use super::{auth::User, AppState};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", routing::get(get_health))
        .route("/user", routing::get(get_user))
        .route("/metadata/:id", routing::get(get_metadata))
        .route("/metadata", routing::post(post_metadata_with_random_id))
        .route("/file/:id", routing::get(get_file).post(post_file))
}

async fn get_health() -> &'static str {
    "OK"
}

async fn get_user(user: User) -> Json<User> {
    Json(user)
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
struct PostMetadataWithRandomIdReq {
    #[serde(flatten)]
    pub req: MetadataCreationReq,
}

#[derive(Serialize)]
struct PostMetadataWithRandomIdResp {
    pub id: String,
}

async fn post_metadata_with_random_id(
    user: User,
    State(state): State<AppState>,
    Json(req): Json<PostMetadataWithRandomIdReq>,
) -> Result<Json<PostMetadataWithRandomIdResp>, (StatusCode, &'static str)> {
    let id = random_string::generate(
        CONFIG.random_uri_length,
        "abcedfghijklmnopqrstuvwxyzABCEDFGHIJKLMNOPQRSTUVWXYZ0123456789",
    );
    let metadata = req.req.into_metadata(user.primary_email);
    s3::upload_metadata(&state.s3_client, &id, &metadata)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to upload metadata to S3");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to upload metadata to S3",
            )
        })?;
    Ok(Json(PostMetadataWithRandomIdResp { id }))
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
