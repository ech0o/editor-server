use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session{
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub token_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

pub fn generate_session_token() -> String{
    let mut bytes = [0u8; 32];

    rand::rng().fill_bytes(&mut bytes);
    hex::encode(&bytes)
}

pub fn hash_session_token(token: &str) -> Vec<u8>{
    Sha256::digest(token.as_bytes()).to_vec()
}

pub struct SessionStore {
    pool: PgPool
}

impl SessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool: pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
    ) -> Result<(Session, String), sqlx::Error> {
        let session_id = Uuid::new_v4();

        let token = generate_session_token();
        let token_hash = hash_session_token(&token);

        let session = sqlx::query_as!(Session,
            r#"
        INSERT INTO sessions (
            id,
            user_id,
            token_hash,
            expires_at
        )
        VALUES (
            $1,
            $2,
            $3,
            NOW() + INTERVAL '30 days'
        )
        RETURNING
            id,
            user_id,
            token_hash,
            expires_at,
            created_at
        "#,
            session_id,
            user_id,
            token_hash
        )
            .fetch_one(&self.pool)
            .await?;
        Ok((session, token))
    }

    pub async fn find_by_token(&self, token: &str) -> Result<Option<Session>, sqlx::Error> {
        let token_hash = hash_session_token(token);

        sqlx::query_as!(Session,
            r#"
                SELECT
                    id,
                    user_id,
                    token_hash,
                    expires_at,
                    created_at
                FROM sessions
                WHERE token_hash = $1
                    AND expires_at > NOW()
                "#,
            token_hash
        )
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn delete_by_token(&self, token: &str) -> Result<bool, sqlx::Error> {
        let token_hash = hash_session_token(token);
        let res = sqlx::query!(
            r#"
                DELETE FROM sessions
                WHERE token_hash = $1
            "#,
            token_hash
        ).execute(&self.pool).await?;
        Ok(res.rows_affected() == 1)
    }
}