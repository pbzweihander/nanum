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
        .route(
            "/metadata/:id",
            routing::get(get_metadata).post(post_metadata),
        )
        .route("/metadata", routing::post(post_metadata_with_random_id))
        .route("/file/:id/:seq", routing::get(get_file).post(post_file))
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
struct PostMetadataReq {
    #[serde(flatten)]
    pub req: MetadataCreationReq,
}

#[derive(Serialize)]
struct PostMetadataResp {
    pub id: String,
}

async fn upload_metadata(
    s3_client: &aws_sdk_s3::Client,
    id: &str,
    user: User,
    req: MetadataCreationReq,
) -> Result<(), (StatusCode, &'static str)> {
    let metadata = req.into_metadata(user.primary_email);
    s3::upload_metadata(s3_client, id, &metadata)
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

async fn post_metadata(
    Path(id): Path<String>,
    user: User,
    State(state): State<AppState>,
    Json(req): Json<PostMetadataReq>,
) -> Result<Json<PostMetadataResp>, (StatusCode, &'static str)> {
    upload_metadata(&state.s3_client, &id, user, req.req).await?;
    Ok(Json(PostMetadataResp { id }))
}

async fn post_metadata_with_random_id(
    user: User,
    State(state): State<AppState>,
    Json(req): Json<PostMetadataReq>,
) -> Result<Json<PostMetadataResp>, (StatusCode, &'static str)> {
    let id = random_string::generate(
        CONFIG.random_uri_length,
        "abcedfghijklmnopqrstuvwxyz0123456789",
    );
    upload_metadata(&state.s3_client, &id, user, req.req).await?;
    Ok(Json(PostMetadataResp { id }))
}

async fn get_file(
    Path((id, seq)): Path<(String, usize)>,
    State(state): State<AppState>,
) -> Result<StreamBody<ByteStream>, (StatusCode, &'static str)> {
    let file = s3::get_file(&state.s3_client, &id, seq)
        .await
        .map_err(|error| {
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
    Path((id, seq)): Path<(String, usize)>,
    State(state): State<AppState>,
    body: Bytes,
) -> Result<(), (StatusCode, &'static str)> {
    s3::upload_file(&state.s3_client, &id, seq, body.to_vec())
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
