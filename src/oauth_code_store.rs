use sha2::{Digest, Sha256};
use crate::authenticated::generate_code;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct OauthCodeStore {
    pub pool: PgPool,
}

impl OauthCodeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_oauth_code(&self, user_id: Uuid) -> anyhow::Result<String> {
        let (code, hash) = generate_code();
        sqlx::query!(
            r#"
            INSERT INTO oauth_codes (code_hash, user_id, expires_at)
            VALUES ($1, $2, NOW() + INTERVAL '60 seconds')
            "#,
            hash,
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(code)
    }

    pub async fn exchange_code(&self, code: &str) -> anyhow::Result<Option<Uuid>> {
        let code_hash = Sha256::digest(code.as_bytes());
        let mut tx = self.pool.begin().await?;
        let user_id = sqlx::query_scalar!(
            r#"
             UPDATE oauth_codes
             SET used_at = NOW()
             WHERE code_hash = $1
              AND used_at is NULL
             AND  expires_at > NOW()
             RETURNING user_id
"#,
            code_hash.as_slice()
        )
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(user_id)
    }
}
