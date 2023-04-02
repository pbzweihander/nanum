mod config;
mod handler;
mod s3;
mod types;
mod utils;

use anyhow::{Context, Result};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::CONFIG;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal received, starting graceful shutdown");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    let aws_config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let http_client = reqwest::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap();

    let router = crate::handler::create_router(s3_client, http_client);

    let listen_addr = CONFIG
        .listen_addr
        .parse()
        .context("failed to parse listen addr")?;

    tracing::info!(%listen_addr, "starting http server...");

    axum::Server::bind(&listen_addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("failed to serve HTTP")?;

    Ok(())
}
