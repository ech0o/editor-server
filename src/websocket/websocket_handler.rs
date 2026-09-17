use crate::error::ApiError;
use crate::jobs::JobStore;
use crate::jwt_service::{Claims, create_ws_ticket};
use crate::kafka::JobEvent;
use crate::middleware::AuthUser;
use crate::state::AppState;
use crate::websocket::manager::WsManager;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{FromRequestParts, Path, State, WebSocketUpgrade};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, RequestPartsExt, Router};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

pub async fn handle_socket(mut socket: WebSocket, manager: Arc<WsManager>, job_id: Uuid) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<JobEvent>(16);
    manager.add(job_id, tx).await;
    while let Some(event) = rx.recv().await {
        let text = serde_json::to_string(&event).unwrap();

        if socket.send(Message::Text(text.into())).await.is_err() {
            break;
        }
        if event.status.is_finished() {
            break;
        }
    }
    manager.remove(job_id).await;
}

#[derive(Debug, Deserialize,Serialize)]
pub struct CreateWsTicketRequest {
    pub job_id: Uuid,
}

#[derive(Debug, Serialize,Deserialize)]
pub struct CreateWsTicketResponse {
    pub ticket: String,
}

// #[derive(Debug, Clone)]
// pub struct AuthJwtUser {
//     pub id: Uuid,
// }
//
// impl<S> FromRequestParts<S> for AuthJwtUser
// where
//     S: Send + Sync,
// {
//     type Rejection = ApiError;
//
//     async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
//         let auth = parts
//             .headers
//             .get("Authorization")
//             .and_then(|v| v.to_str().ok())
//             .ok_or(ApiError::Unauthorized)?;
//
//         let token = auth
//             .strip_prefix("Bearer ")
//             .ok_or(ApiError::Unauthorized)?
//             .to_string();
//
//         let Extension(state) = parts
//             .extract::<Extension<Arc<AppState>>>()
//             .await
//             .map_err(|err| ApiError::Internal(err.into()))?;
//
//         let claims = decode_jwt(token.as_str(), state.jwt.secret.as_bytes())
//             .map_err(|err| ApiError::Unauthorized)?;
//         Ok(AuthJwtUser { id: claims.sub })
//     }
// }

pub fn decode_jwt(
    token: &str,
    secret: &[u8],
) -> anyhow::Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

