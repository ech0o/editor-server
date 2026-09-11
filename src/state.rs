pub(crate) use std::sync::Arc;

use crate::{jobs::JobStore, producer::KafkaProducer};
use crate::apikey::ApiKey;
use crate::apikey_store::ApikeyStore;
use crate::config::AppConfig;
use crate::job_service::JobService;
use crate::session_store::SessionStore;
use crate::user_store::UserStore;

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
    pub job_service: Arc<JobService>
    // pub db:Database
}

impl AppState{
    pub fn new(
        job_store: Arc<JobStore>,
        kafka_producer: KafkaProducer,
        api_keys: Arc<ApikeyStore>,
        app_config: AppConfig,
        users: Arc<UserStore>,
        sessions: Arc<SessionStore>,
        job_service: Arc<JobService>
        // metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            // semaphore: Arc::new(Semaphore::new(4)),
            jobs: job_store,
            kafka: kafka_producer,
            api_keys,
            config: app_config,
            users,
            sessions,
            job_service
            // metrics,
        }
    }
}