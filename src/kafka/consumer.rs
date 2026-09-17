use crate::kafka::JobEvent;
use crate::websocket::manager::WsManager;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use std::sync::Arc;

use sqlx::types::JsonValue;

pub struct KafkaConsumer {
    pub broker: String,
    pub group_id: String,
}


impl KafkaConsumer {
    pub fn create_consumer(broker: String, group_id: String) -> anyhow::Result<StreamConsumer> {
        let consumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", broker)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()?;
        Ok(consumer)
    }
}

pub async fn consume_job_event(consumer: StreamConsumer, manager: Arc<WsManager>)->anyhow::Result<()> {
    

    while let Some(msg) = consumer.recv().await.ok() {
        let payload = msg.payload().unwrap();
        let event: JobEvent = serde_json::from_slice(payload)?;

        tracing::debug!(event=?event,"event sent");
        manager.send(event).await;
        consumer.commit_message(&msg, CommitMode::Async)?;
    }
    Ok(())
}
