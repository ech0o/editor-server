use std::collections::HashMap;
use std::sync::Arc;
use anyhow::anyhow;
use axum::{Extension, Json};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::Response;
use jsonwebtoken::{DecodingKey, Validation};
use serde::Deserialize;
use uuid::Uuid;
use crate::error::ApiError;
use crate::jwt_service::{create_ws_ticket, WsClaims};
use crate::middleware::AuthUser;
use crate::state::AppState;
use crate::websocket::websocket_handler::{handle_socket, CreateWsTicketRequest, CreateWsTicketResponse};

#[derive(Debug,Deserialize)]
pub struct WsQuery{
    pub ticket:String,
}

pub async fn job_ws(
    ws:WebSocketUpgrade,
    State(state):State<Arc<AppState>>,
    Path(job_id):Path<Uuid>,
    Query(query):Query<WsQuery>,
)->anyhow::Result<Response,ApiError>{
    let token_data = jsonwebtoken::decode::<WsClaims>(
        &query.ticket,
        &DecodingKey::from_secret(state.jwt.secret.as_bytes()),
        &Validation::default(),
    )
        .map_err(|_|ApiError::Unauthorized)?;
    let claims = token_data.claims;
    if claims.job_id!=job_id{
        return Err(ApiError::Unauthorized);
    }
    let ws_manager = state.ws_manager.clone();
    Ok(ws.on_upgrade(move |socket|{
        handle_socket(socket,ws_manager,job_id)
    }))
}

pub async fn create_ws_ticket_route(
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWsTicketRequest>,
) -> anyhow::Result<Json<CreateWsTicketResponse>, ApiError> {
    let job = state
        .jobs
        .get(req.job_id)
        .await?
        .ok_or(ApiError::JobNotFound)?;
    let Some(user_id) = job.user_id else {
        return Err(ApiError::Internal(anyhow!("job user_id is none")));
    };
    if user_id != user.user_id {
        return Err(ApiError::JobNotFound);
    }

    let ticket = create_ws_ticket(user.user_id, req.job_id, state.jwt.secret.as_bytes())
        .map_err(|_| ApiError::Internal(anyhow!("jwt ticket not found")))?;
    Ok(Json(CreateWsTicketResponse { ticket }))
}