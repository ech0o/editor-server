use crate::state::AppState;
use axum::extract::State;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiKeyIdentity {
    pub api_key_id: Uuid,
}
