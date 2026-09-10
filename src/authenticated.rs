use crate::state::AppState;
use axum::extract::State;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Authenticated {
    pub api_key_id: Uuid,
}
