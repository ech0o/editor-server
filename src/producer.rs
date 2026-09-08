use std::time::Duration;

use anyhow::anyhow;
use rdkafka::{ClientConfig, producer::{FutureProducer, FutureRecord}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct KafkaProducer {
    pub producer: FutureProducer,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct JobMessage{
    pub job_id: Uuid,
}

impl KafkaProducer {
    pub fn new(broker: &str) -> anyhow::Result<Self> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", broker)
            .create()?;
        tracing::info!("Kafka producer created");
        Ok(Self { producer })
    }

    pub async fn send_job(&self, job: &JobMessage) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(job)?;
        self.producer
            .send(
                FutureRecord::to("judge.jobs")
                    .key(&job.job_id.to_string())
                    .payload(&payload),
                Duration::from_secs(5),
            )
            .await
            .map_err(|(err, _)| anyhow!(err))?;
        Ok(())
    }
}