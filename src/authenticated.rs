use crate::state::AppState;
use axum::extract::State;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiKeyIdentity {
    pub api_key_id: Uuid,
    pub user_id: Uuid,
}



pub fn generate_code() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let code = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let hash = Sha256::digest(code.as_bytes());
    (code, hash.to_vec())
}


