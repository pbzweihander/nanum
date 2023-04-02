use std::str::FromStr;

use axum::{extract::Path, http::StatusCode, routing, Router, TypedHeader};
use headers::ContentType;
use include_dir::{include_dir, Dir};

const STATIC_FILES_DIR: Dir<'static> = include_dir!("frontend/dist");

pub fn create_router() -> Router {
    Router::new().route("/:filename", routing::get(get_static_file))
}

async fn get_static_file(
    Path(filename): Path<String>,
) -> Result<(TypedHeader<ContentType>, &'static [u8]), StatusCode> {
    if filename.len() > 1000 {
        return Err(StatusCode::NOT_FOUND);
    }

    let file = STATIC_FILES_DIR
        .get_file(&filename)
        .ok_or(StatusCode::NOT_FOUND)?;

    let ext = if let Some((_, ext)) = filename.rsplit_once('.') {
        ext
    } else {
        // If no extension, not found.
        return Err(StatusCode::NOT_FOUND);
    };

    let content_type = match ext {
        "js" => ContentType::from(mime::APPLICATION_JAVASCRIPT_UTF_8),
        "css" => ContentType::from(mime::TEXT_CSS_UTF_8),
        "png" => ContentType::png(),
        "json" => ContentType::json(),
        "ico" => ContentType::from(mime::Mime::from_str("image/x-icon").unwrap()),
        "wasm" => ContentType::from(mime::Mime::from_str("application/wasm").unwrap()),
        _ => {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    Ok((TypedHeader(content_type), file.contents()))
}
