use axum::{Json, Router, extract::{Path, State, rejection::JsonRejection}, http::StatusCode, routing::{get, post}};
use uuid::Uuid;

use crate::{error::ApiError, jobs::Job, model::{JobIdResponse, JobResponse, RunRequest, RunStatus}, producer::JobMessage, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        // .route("/health", get(health))
        // // .route("/container", post(create_container))
        // // .route("/exec", get(exec))
        // .route("/mount", get(workspace))
        .route("/run", post(run_handle))
        .route("/run/{job_id}", get(get_job))
}

async fn run_handle(
    State(state): State<AppState>,
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
    state.kafka.send_job(&job_msg).await?;
    Ok(Json(JobIdResponse { job_id }))
}

async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobResponse>, ApiError> {
    let job = state.jobs.get(job_id).await?.ok_or(ApiError::JobNotFound)?;

    Ok(Json(job.into()))
}