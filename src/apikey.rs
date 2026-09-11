use chrono::{DateTime, Utc};
use rand::Rng;
use sha2::Digest;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub user_id: Option<Uuid>,
    pub key_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub fn generate_api_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("sk_live_{}", hex::encode(bytes))
}

pub fn hash_api_key(key: &str) -> Vec<u8> {
    sha2::Sha256::digest(key.as_bytes()).to_vec()
}
