use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::handler::HandlerWithoutStateExt;
use axum::http::{HeaderValue, Method, header};
use axum::middleware::from_fn_with_state;
use axum_governor::{
    GovernorConfigBuilder, GovernorLayer, PeerIp, Quota,
    extractor::Extension as GonvernorExtension, nz,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::apikey_store::ApikeyStore;
use crate::authenticated::ApiKeyIdentity;
use crate::job_service::JobService;
use crate::middleware::{auth_middleware, require_session};
use crate::routes::{
    api_job_router, api_router, auth_router, protected_router, web_job_router, web_router,
};
use crate::session_store::{AuthenticatedUser, SessionStore};
use crate::user_store::UserStore;
use crate::{jobs::JobStore, producer::KafkaProducer, state::AppState};

mod jobs;

mod producer;

mod state;

mod routes;

mod model;

mod apikey;
mod apikey_store;
mod authenticated;
mod config;
mod error;
mod github;
mod job_owner;
mod job_service;
mod middleware;
mod session_store;
mod user_store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    // dotenvy::dotenv()?;
    let db_addr = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:example@localhost:5432/postgres".to_string());
    let db = PgPool::connect(db_addr.as_str()).await?;
    let kafka_addr =
        std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_string());
    tracing::info!("Kafka brokers is {}", kafka_addr);
    tracing::info!("db_addr: {:?}", db_addr);
    let kafka = KafkaProducer::new(kafka_addr.as_str())?;

    let config = config::AppConfig::from_env()?;
    let jobs = Arc::new(JobStore::new(db.clone()));
    let api_keys = Arc::new(ApikeyStore::new(db.clone()));
    let users = Arc::new(UserStore::new(db.clone()));
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>()?)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    let global_ip_config = GovernorConfigBuilder::default()
        .with_extractor(PeerIp::default())
        .expect_connect_info()
        .quota_default(Quota::requests_per_second(nz!(20u32)))
        .finish()?;

    let web_rate_limit_config = GovernorConfigBuilder::default()
        .with_extractor(GonvernorExtension::<AuthenticatedUser>::new())
        .quota_default(Quota::requests_per_second(nz!(5u32)))
        .finish()?;
    let web_job_rate_limit_config = GovernorConfigBuilder::default()
        .with_extractor(GonvernorExtension::<AuthenticatedUser>::new())
        .quota_default(Quota::requests_per_second(nz!(30u32)))
        .finish()?;

    let session = Arc::new(SessionStore::new(db.clone()));

    let job_service = JobService {
        jobs: jobs.clone(),
        producer: kafka.clone(),
    };
    let state = Arc::new(AppState::new(
        jobs,
        kafka,
        api_keys,
        config,
        users,
        session,
        Arc::new(job_service),
    ));

    let api_key_config = GovernorConfigBuilder::default()
        .with_extractor(GonvernorExtension::<ApiKeyIdentity>::new())
        .quota_default(Quota::requests_per_second(nz!(5u32)))
        .finish()?;
    let api_key_job_config = GovernorConfigBuilder::default()
        .with_extractor(GonvernorExtension::<ApiKeyIdentity>::new())
        .quota_default(Quota::requests_per_second(nz!(30u32)))
        .finish()?;
    let api_routes = api_router()
        .layer(GovernorLayer::new(api_key_config))
        .layer(from_fn_with_state(state.clone(), auth_middleware));

    let api_job_routes = api_job_router()
        .layer(GovernorLayer::new(api_key_job_config))
        .layer(from_fn_with_state(state.clone(), auth_middleware));

    let web_routes = web_router()
        .layer(GovernorLayer::new(web_rate_limit_config))
        .layer(from_fn_with_state(state.clone(), require_session));

    let web_job_routes = web_job_router()
        .layer(GovernorLayer::new(web_job_rate_limit_config))
        .layer(from_fn_with_state(state.clone(), require_session));

    let auth_routes = auth_router();
    let protected_routes = protected_router(state.clone());
    let router = Router::new()
        .layer(DefaultBodyLimit::max(64 * 1024))
        // .merge(routes::router())
        .merge(api_routes)
        .merge(api_job_routes)
        .merge(auth_routes)
        .merge(web_routes)
        .merge(web_job_routes)
        .merge(protected_routes)
        .layer(GovernorLayer::new(global_ip_config))
        .layer(cors)
        .with_state(state);
    let listener = TcpListener::bind("0.0.0.0:4000").await?;
    tracing::info!("Listening on http://0.0.0.0:4000");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
