use crate::apikey::hash_api_key;
use crate::authenticated::Authenticated;
use crate::github::generate_oauth_state;
use crate::jobs::JobStoreError;
use crate::model::{GithubCallback, GithubTokenResponse, GithubUser};
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
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use url::Url;
use uuid::Uuid;

const MAX_CONCURRENT_RUN_REQUESTS: usize = 32;

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

    request.extensions_mut().insert(Authenticated {
        api_key_id: api_key.id,
    });
    Ok(next.run(request).await)
}
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // .route("/health", get(health))
        // // .route("/container", post(create_container))
        // // .route("/exec", get(exec))
        // .route("/mount", get(workspace))
        .route("/run/{job_id}", get(get_job))
}

pub fn run_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/run",
        post(run_handle).layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_RUN_REQUESTS)),
    )
}

pub fn auth_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/github", get(github_login))
        .route("/auth/github/callback", get(github_callback))
}

async fn run_handle(
    Extension(auth): Extension<Authenticated>,
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

    let job = Job {
        id: Uuid::new_v4(),
        language: "rust".to_string(),
        code: request.code,
        status: RunStatus::Accepted,
        stdout: None,
        stderr: None,
        exit_code: None,
        created_at: None,
        worker_id: None,
        heartbeat_at: None,
    };
    let job_id = job.id;
    let job_msg = JobMessage { job_id };
    state.jobs.create(&job).await?;
    if let Err(err) = state.kafka.send_job(&job_msg).await {
        tracing::error!(
            job_id=%job_id,
            error=%err,
            "failed to send job to kafka"
        );

        let _ = state.jobs.mark_failed_if_queued(job_id).await;
        return Err(ApiError::Internal(anyhow!("failed to send job to kafka")));
    }
    Ok(Json(JobIdResponse { job_id }))
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state.jobs.get(job_id).await?.ok_or(ApiError::JobNotFound)?;

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

    let client = reqwest::Client::new();

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

    let github_user = client
        .get("https://api.github.com/user")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .header(reqwest::header::USER_AGENT, "code-execution-server")
        .bearer_auth(&token.access_token)
        .send()
        .await
        .map_err(ApiError::GithubRequest)?
        .error_for_status()
        .map_err(ApiError::GithubRequest)?
        .json::<GithubUser>()
        .await
        .map_err(ApiError::GithubRequest)?;

    let user = state.users.find_or_create_github_user(&github_user).await?;

    let (_session,session_token) = state
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

    let jar = jar.remove(Cookie::from("oauth_state")).add(session_cookie);
    Ok((jar, Redirect::to("/")))
}
