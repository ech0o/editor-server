use crate::apikey::{ApiKey, generate_api_key, hash_api_key};
use sqlx::PgPool;
use std::ptr::hash;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApikeyStore {
    pool: PgPool,
}

impl ApikeyStore {
    pub fn new(pool: PgPool) -> ApikeyStore {
        Self { pool }
    }

    pub async fn create(&self, name: &str) -> anyhow::Result<(ApiKey, String), sqlx::Error> {
        let raw_key = generate_api_key();
        let key_hash = hash_api_key(&raw_key);
        let id = Uuid::new_v4();

        let api_key = sqlx::query_as!(
            ApiKey,
            r#"
                INSERT INTO api_keys(
                    id,
                    name,
                    key_hash
                )
                VALUES ($1, $2, $3)
                RETURNING
                    id,
                name,
                key_hash,
                created_at,
                revoked_at
            "#,
            id,
            name,
            &key_hash,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok((api_key, raw_key))
    }

    pub async fn find_by_hash(&self, hash: &[u8]) -> anyhow::Result<Option<ApiKey>, sqlx::Error> {
        sqlx::query_as!(
            ApiKey,
            r#"
                SELECT id, name, key_hash, created_at, revoked_at
                FROM api_keys
                WHERE key_hash = $1
                    AND revoked_at IS NULL
            "#,
            hash
        )
            .fetch_optional(&self.pool)
        .await
    }

    pub async fn revoke(&self, id:Uuid) -> anyhow::Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
                UPDATE  api_keys
                SET revoked_at = NOW()
                WHERE id = $1
                    AND revoked_at IS NULL
             "#,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
