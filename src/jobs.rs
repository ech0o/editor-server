use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::RunStatus;

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


impl JobStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, job: &Job) -> anyhow::Result<()> {
        sqlx::query(
            r#"
INSERT INTO jobs (id, language, code,status,stdout,
            stderr,
            exit_code)
VALUES ($1, $2, $3, $4, $5, $6, $7)
"#,
        )
        .bind(job.id)
        .bind("rust")
        .bind(&job.code)
        .bind(job.status.to_string())
        .bind(&job.stdout)
        .bind(&job.stderr)
        .bind(job.exit_code)
        .execute(&self.pool)
        .await?;
        Ok(())
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
}