use crate::apikey::{ApiKey, hash_api_key};
use crate::authenticated::ApiKeyIdentity;
use crate::github::generate_oauth_state;
use crate::job_owner::JobOwner;
use crate::jobs::{JobStoreError, NewJob};
use crate::middleware::require_session;
use crate::model::{
    ApiKeysResponse, CreateApiKeyRequest, CreateApiKeyResponse, GithubCallback,
    GithubTokenResponse, GithubUser,
};
use crate::session_store::AuthenticatedUser;
use crate::{
    error::ApiError,
    jobs::Job,
    model::{JobIdResponse, JobResponse, RunRequest, RunStatus},
    producer::JobMessage,
    state::AppState,
};
use anyhow::anyhow;
use axum::extract::{Query, Request};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{Redirect, Response};
use axum::routing::delete;
use axum::{
    Extension, Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use axum_extra::TypedHeader;
use axum_extra::extract::cookie::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use headers::Authorization;
use headers::authorization::Bearer;
use rdkafka::client;
use std::sync::Arc;
use tower::ServiceExt;
use tower::limit::ConcurrencyLimitLayer;
use url::Url;
use uuid::Uuid;

const MAX_CONCURRENT_RUN_REQUESTS: usize = 32;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
    // .route("/health", get(health))
    // // .route("/container", post(create_container))
    // // .route("/exec", get(exec))
    // .route("/mount", get(workspace))
    // .route("/run/{job_id}", get(get_job))
}

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/run",
            post(run_handle).layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_RUN_REQUESTS)),
        )
}
pub fn api_job_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/run/{job_id}",
            get(get_job).layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_RUN_REQUESTS)),
        )
}
pub fn web_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/web/run",
            post(web_run).layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_RUN_REQUESTS)),
        )
}
pub fn web_job_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/web/run/{id}",
            get(web_get_job).layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_RUN_REQUESTS)),
        )
}

pub fn protected_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api-keys", post(create_api_keys))
        .route("/api-keys", get(list_api_keys))
        .route("/api-keys/{id}", delete(revoke_api_keys))
        .layer(from_fn_with_state(state.clone(), require_session))
}

pub fn auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/github", get(github_login))
        .route("/auth/github/callback", get(github_callback))
}

async fn run_handle(
    Extension(api_key): Extension<ApiKeyIdentity>,
    State(state): State<Arc<AppState>>,
    results: Result<Json<RunRequest>, JsonRejection>,
) -> Result<Json<JobIdResponse>, ApiError> {
    // let _permit = match state.semaphore.try_acquire(){
    //     Ok(permit) => permit,
    //     Err(_)=>{
    //         return Err(ApiError::TooManyRequests)
    //     }
    // };
    let Json(request) = results.map_err(|err| match err.status() {
        StatusCode::PAYLOAD_TOO_LARGE => {
            tracing::error!(
                error=%err,
                "Invalid JSON request"
            );
            ApiError::PayloadTooLarge
        }
        _ => {
            println!("{}", err.status());
            tracing::error!(
                error=%err,
                "Invalid JSON request"
            );
            ApiError::InvalidJson
        }
    })?;

    let res = state
        .job_service
        .submit(
            JobOwner::Apikey {
                api_key_id: api_key.api_key_id,
            },
            request,
        )
        .await?;
    Ok(Json(res))
}

pub async fn web_run(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<Arc<AppState>>,
    req: Result<Json<RunRequest>, JsonRejection>,
) -> anyhow::Result<Json<JobIdResponse>, ApiError> {
    let Json(request) = req.map_err(|err| match err.status() {
        StatusCode::PAYLOAD_TOO_LARGE => {
            tracing::error!(
                error=%err,
                "Invalid JSON request"
            );
            ApiError::PayloadTooLarge
        }
        _ => {
            println!("{}", err.status());
            tracing::error!(
                error=%err,
                "Invalid JSON request"
            );
            ApiError::InvalidJson
        }
    })?;
    let job_service = state.job_service.clone();
    let owner = crate::job_owner::JobOwner::User {
        user_id: user.user_id,
    };
    let response = job_service.submit(owner, request).await?;
    Ok(Json(response))
}

async fn get_job(
    Extension(api_key): Extension<ApiKeyIdentity>,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state
        .jobs
        .find_by_api_key(job_id, api_key.api_key_id)
        .await?
        .ok_or(ApiError::JobNotFound)?;
    Ok(Json(job.into()))
}

pub async fn github_login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> (CookieJar, Redirect) {
    let oauth_state = generate_oauth_state();

    let mut url =
        Url::parse("http://github.com/login/oauth/authorize").expect("valid github authorize url");

    url.query_pairs_mut()
        .append_pair("client_id", &state.config.github.client_id)
        .append_pair("redirect_uri", &state.config.github.redirect_url)
        .append_pair("scope", "read:user")
        .append_pair("state", &oauth_state);

    let cookie = Cookie::build(("oauth_state", oauth_state))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.config.cookie_secure)
        .build();

    (jar.add(cookie), Redirect::temporary(url.as_str()))
}

pub async fn github_callback(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(query): Query<GithubCallback>,
) -> anyhow::Result<(CookieJar, Redirect), ApiError> {
    let cookie_state = jar.get("oauth_state").ok_or(ApiError::Unauthorized)?;

    if cookie_state.value() != query.state {
        return Err(ApiError::Unauthorized);
    }
    tracing::info!(proxy=%state.config.reqwest_proxy,"proxy setting");
    let proxy = match reqwest::Proxy::all(state.config.reqwest_proxy.clone()) {
        Ok(proxy) => proxy,
        Err(err) => {
            tracing::error!(error=%err, "failed to create reqwest proxy");
            return Err(ApiError::GithubRequest(err));
        }
    };

    let client = reqwest::Client::builder().proxy(proxy).build();

    let client = match client {
        Ok(client) => client,
        Err(err) => {
            tracing::error!(error=%err, "failed to build reqwest client");
            return Err(ApiError::GithubRequest(err));
        }
    };

    let res = client.get("https://www.google.com").send().await?;
    println!("Status: {}", res.status());

    let token = client
        .post("https://github.com/login/oauth/access_token")
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&serde_json::json!({
            "client_id":state.config.github.client_id,
            "client_secret":state.config.github.client_secret,
            "code":query.code,
            "redirect_uri":state.config.github.redirect_url,
        }))
        .send()
        .await
        .map_err(ApiError::GithubRequest)?
        .error_for_status()
        .map_err(ApiError::GithubRequest)?
        .json::<GithubTokenResponse>()
        .await
        .map_err(ApiError::GithubRequest)?;

    tracing::info!("token:{:?}", token);
    let github_user = client
        .get("https://api.github.com/user")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .header(reqwest::header::USER_AGENT, "code-execution-server")
        .bearer_auth(&token.access_token)
        .send()
        .await
        .map_err(ApiError::GithubRequest)?
        .error_for_status()?
        .json::<GithubUser>()
        .await
        .map_err(ApiError::GithubRequest)?;
    tracing::info!("github_user:{:?}", github_user);
    // tracing::info!(headers=?res.headers(), "github_user_response:");
    // let body = res.text().await.map_err(ApiError::GithubRequest)?;
    // tracing::info!("github_user_response_body:{}", body);
    let user = state.users.find_or_create_github_user(&github_user).await?;

    let (_session, session_token) = state
        .sessions
        .create(user.id)
        .await
        .map_err(ApiError::DataBase)?;
    let session_cookie = Cookie::build(("session", session_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(state.config.cookie_secure)
        .max_age(time::Duration::days(30))
        .build();
    let is_login_cookie = Cookie::build(("is_login", "true"))
        .path("/")
        .http_only(false)
        .same_site(SameSite::Lax)
        .secure(state.config.cookie_secure)
        .max_age(time::Duration::days(30))
        .build();

    let jar = jar
        .remove(Cookie::from("oauth_state"))
        .add(session_cookie)
        .add(is_login_cookie);
    Ok((
        jar,
        Redirect::to(state.config.redirect_frontend_url.as_str()),
    ))
}

pub async fn create_api_keys(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> anyhow::Result<Json<CreateApiKeyResponse>, ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::InvalidRequest(anyhow!("name cannot be empty")));
    }

    if name.len() > 100 {
        return Err(ApiError::InvalidRequest(anyhow!("name too long")));
    }

    let (api_key, raw_key) = state
        .api_keys
        .create(user.user_id, name)
        .await
        .map_err(ApiError::DataBase)?;

    Ok(Json(CreateApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        key: raw_key,
    }))
}

pub async fn list_api_keys(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<Arc<AppState>>,
) -> anyhow::Result<Json<Vec<ApiKeysResponse>>, ApiError> {
    let keys = state
        .api_keys
        .list_by_user(user.user_id)
        .await
        .map_err(ApiError::DataBase)?;
    Ok(Json(keys))
}

pub async fn revoke_api_keys(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> anyhow::Result<StatusCode, ApiError> {
    let revoked = state
        .api_keys
        .revoke(id, user.user_id)
        .await
        .map_err(ApiError::DataBase)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn web_get_job(
    Extension(user): Extension<AuthenticatedUser>,
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state
        .jobs
        .find_for_user(job_id, user.user_id)
        .await?
        .ok_or(ApiError::JobNotFound)?;
    Ok(Json(JobResponse::from(job)))
}
