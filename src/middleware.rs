use crate::session_store::AuthenticatedUser;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use std::sync::Arc;
use axum_extra::TypedHeader;
use headers::Authorization;
use headers::authorization::Bearer;
use crate::apikey::hash_api_key;
use crate::authenticated::ApiKeyIdentity;

pub async fn require_session(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> anyhow::Result<Response, StatusCode> {
    let cookie = jar.get("session").ok_or(StatusCode::UNAUTHORIZED)?;

    let token = cookie.value();

    let session = state
        .sessions
        .find_by_token(token)
        .await
        .map_err(|err| {
            tracing::error!(error=%err,"failed to find session");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(AuthenticatedUser {
        user_id: session.user_id,
    });
    Ok(next.run(request).await)
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> anyhow::Result<Response, StatusCode> {
    let key = auth.token();

    let key_hash = hash_api_key(key);

    let api_key = state
        .api_keys
        .find_by_hash(&key_hash)
        .await
        .map_err(|err| {
            tracing::error!(
                error = %err,
                "failed to lookup api key"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let Some(user_id) = api_key.user_id else{
        tracing::error!("api key user_id does not exist");
        return Err(StatusCode::UNAUTHORIZED)
    };
    request.extensions_mut().insert(ApiKeyIdentity {
        api_key_id: api_key.id,
        user_id
    });
    Ok(next.run(request).await)
}