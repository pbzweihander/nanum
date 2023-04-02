mod api;
mod auth;
mod statics;

use axum::{
    http::{header, Request},
    middleware::Next,
    response::{Html, Response},
    routing, Router,
};

use self::auth::User;

#[derive(Clone)]
pub struct AppState {
    s3_client: aws_sdk_s3::Client,
    http_client: reqwest::Client,
    oauth_client: oauth2::basic::BasicClient,
}

pub fn create_router(s3_client: aws_sdk_s3::Client, http_client: reqwest::Client) -> Router {
    let oauth_client = auth::create_oauth_client();

    let state = AppState {
        s3_client,
        http_client,
        oauth_client,
    };

    let api = api::create_router();
    let auth = auth::create_router();
    let statics = statics::create_router();

    Router::new()
        .nest("/api", api)
        .nest("/auth", auth)
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .route("/", routing::get(get_frontend_index))
        .route("/:id", routing::get(get_frontend_download))
        .nest("/static", statics)
        .layer(axum::middleware::from_fn(server_header_middleware))
}

async fn get_frontend_index(_user: User) -> Html<&'static str> {
    Html(include_str!("../../../frontend/dist/index.html"))
}

async fn get_frontend_download() -> Html<&'static str> {
    Html(include_str!("../../../frontend/dist/index.html"))
}

async fn server_header_middleware<B>(req: Request<B>, next: Next<B>) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        header::SERVER,
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .parse()
            .unwrap(),
    );
    resp
}
