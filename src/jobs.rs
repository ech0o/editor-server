use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::RunStatus;

const JOB_ADMISSION_LOCK: i64 = 0x4A4F425F41434D;
const MAX_ACTIVE_JOBS: i64 = 100;
#[derive(Debug)]
pub struct JobStore {
    // pub jobs: RwLock<HashMap<Uuid, JobState>>,
    pub pool: PgPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub language: String,
    pub code: String,
    pub status: RunStatus,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub worker_id: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum JobStoreError {
    #[error("job capacity exceeded")]
    CapacityExceeded,

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl JobStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, job: &Job) -> anyhow::Result<Job, JobStoreError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            SELECT pg_advisory_xact_lock($1);"#,
            JOB_ADMISSION_LOCK
        )
        .execute(&mut *tx)
        .await?;

        let active_count = sqlx::query_scalar!(
            r#"
             SELECT COUNT(*) FROM jobs
             WHERE status IN ('Queued', 'Running')
             "#
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);

        if active_count >= MAX_ACTIVE_JOBS {
            return Err(JobStoreError::CapacityExceeded.into());
        }

        let result = sqlx::query_as!(
            Job,
            r#"
            INSERT INTO jobs (
                          id,
                          language,
                          code,
                          status,
                          stdout,
                        stderr,
                        exit_code
                        )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                language,
                code,
                status AS "status: RunStatus",
                stdout,
                stderr,
                exit_code,
                created_at,
                heartbeat_at,
                worker_id
            "#,
            job.id,
            "rust",
            job.code,
            job.status.to_string(),
            job.stdout,
            job.stderr,
            job.exit_code,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result)
    }

    pub async fn update_status(&self, id: Uuid, status: RunStatus) -> anyhow::Result<()> {
        let result = sqlx::query(
            r#"
UPDATE jobs
SET status = $1 WHERE id = $2"#,
        )
        .bind(status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await?;
        tracing::info!(result = ?result, "updating job status");
        if result.rows_affected() == 0 {
            anyhow::bail!("job not found: {id}");
        }
        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> anyhow::Result<Option<Job>> {
        let row = sqlx::query_as!(
            Job,
            r#"
        SELECT
            id,
            language,
            code,
            status,
            stdout,
            worker_id,
            stderr,
            exit_code,
            created_at,
            heartbeat_at
        FROM jobs
        WHERE id = $1
        "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(job) = row else {
            return Ok(None);
        };

        Ok(Some(job))
    }

    pub async fn mark_failed_if_queued(&self, job_id: Uuid) -> anyhow::Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            UPDATE jobs
            SET 
                status = 'Failed',
                updated_at = NOW()
            WHERE id = $1
                AND status = 'Queued'
            "#,
            job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
