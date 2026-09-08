use std::sync::Arc;

use axum::http::{HeaderValue, Method};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::{jobs::JobStore, producer::KafkaProducer, state::AppState};

mod jobs;

mod producer;

mod state;

mod routes;

mod model;

mod error;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let db_addr = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:example@localhost:5432/postgres".to_string());
    let db = PgPool::connect(db_addr.as_str()).await?;
    let kafka_addr =
        std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
    tracing::info!("db_addr: {:?}", db_addr);
    let kafka = KafkaProducer::new(kafka_addr.as_str())?;
    let jobs = Arc::new(JobStore::new(db));

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>()?)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    let state = state::AppState::new(jobs, kafka);
    let router = routes::router().layer(cors).merge(routes::router())
        .with_state(state);
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    tracing::info!("Listening on http://0.0.0.0:4000");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}


async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
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
}