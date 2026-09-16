pub(crate) use std::sync::Arc;

use crate::apikey::ApiKey;
use crate::apikey_store::ApikeyStore;
use crate::config::AppConfig;
use crate::job_service::JobService;
use crate::jwt_service::JwtConfig;
use crate::oauth_code_store::OauthCodeStore;
use crate::session_store::SessionStore;
use crate::user_store::UserStore;
use crate::{jobs::JobStore, producer::KafkaProducer};
use crate::websocket::manager::WsManager;

#[derive(Clone)]
pub struct AppState {
    // pub runner: DockerRunner,
    // pub semaphore: Arc<Semaphore>,
    pub jobs: Arc<JobStore>,
    pub kafka: KafkaProducer,
    pub api_keys: Arc<ApikeyStore>,
    pub config: AppConfig,
    pub users: Arc<UserStore>,
    pub sessions: Arc<SessionStore>,
    pub job_service: Arc<JobService>,
    pub oauth_code_store: Arc<OauthCodeStore>,
    pub jwt: Arc<JwtConfig>, 
    pub ws_manager: Arc<WsManager>,
}

impl AppState {
    pub fn new(
        job_store: Arc<JobStore>,
        kafka_producer: KafkaProducer,
        api_keys: Arc<ApikeyStore>,
        app_config: AppConfig,
        users: Arc<UserStore>,
        sessions: Arc<SessionStore>,
        job_service: Arc<JobService>,
        oauth_code_store: Arc<OauthCodeStore>,
        jwt: Arc<JwtConfig>, // metrics: Arc<Metrics>
        ws_manager: Arc<WsManager>,
    ) -> Self {
        Self {
            // semaphore: Arc::new(Semaphore::new(4)),
            jobs: job_store,
            kafka: kafka_producer,
            api_keys,
            config: app_config,
            users,
            sessions,
            job_service,
            oauth_code_store,
            jwt, // metrics,
            ws_manager,
        }
    }
}
