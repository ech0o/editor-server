use std::sync::Arc;

use crate::{jobs::JobStore, producer::KafkaProducer};

#[derive(Clone)]
pub struct AppState {
    // pub runner: DockerRunner,
    // pub semaphore: Arc<Semaphore>,
    pub jobs: Arc<JobStore>,
    pub kafka: KafkaProducer,
    // pub db:Database
}

impl AppState{
    pub fn new(
        job_store: Arc<JobStore>,
        kafka_producer: KafkaProducer,
        // metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            // semaphore: Arc::new(Semaphore::new(4)),
            jobs: job_store,
            kafka: kafka_producer,
            // metrics,
        }
    }
}