use std::sync::Arc;

use sha2::digest::typenum::U;
use sqlx::encode::IsNull::No;
use uuid::Uuid;

use crate::{
    error::ApiError,
    job_owner::JobOwner,
    jobs::{Job, JobStore, NewJob},
    model::{JobIdResponse, RunRequest, RunResponse},
    producer::{JobMessage, KafkaProducer},
};

pub struct JobService {
    pub jobs: Arc<JobStore>,
    pub producer: KafkaProducer,
}

impl JobService {
    pub async fn submit(
        &self,
        owner: JobOwner,
        req: RunRequest,
    ) -> anyhow::Result<JobIdResponse, ApiError> {
        req.validate()?;
        let mut new_job = NewJob {
            id: Uuid::new_v4(),
            user_id: None,
            api_key_id: None,
            language: "rust".to_string(),
            code: req.code,
            status: crate::model::RunStatus::Accepted,
            stdout: None,
            stderr: None,
            exit_code: None,
            created_at: None,
            heartbeat_at: None,
            locked_at: None,
            worker_id: None,
            lock_token: None,
            updated_at: None,
        };
        owner.apply(&mut new_job);
        let job = self.jobs.create(&new_job).await?;
        let job_message = JobMessage { job_id: job.id };
        if let Err(err) = self.producer.send_job(&job_message).await {
            tracing::error!(
                job_id=%job.id,
                error=%err,
                "failed to send job to kafka"
            );
            let _ = self.jobs.mark_failed_if_queued(job.id).await;
            return Err(ApiError::Internal(anyhow::anyhow!(
                "failed to send job to kafka"
            )));
        }
        Ok(JobIdResponse { job_id: job.id })
    }
}
