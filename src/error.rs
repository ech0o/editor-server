use crate::jobs::JobStoreError;
use axum::http::header;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Internal server error")]
    Internal(anyhow::Error),
    #[error("json invalid")]
    InvalidJson,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("too many requests")]
    TooManyRequests,
    #[error("queue closed")]
    QueueClosed,
    #[error("job not found")]
    JobNotFound,
    #[error("job capacity exceeded")]
    CapacityExceeded,
    #[error(transparent)]
    JobStore(JobStoreError),
    #[error("unauthorized")]
    Unauthorized,
    #[error("github request failed：{0}")]
    GithubRequest(#[from] reqwest::Error),
    #[error("database error")]
    DataBase(#[source] sqlx::Error),

    #[error(transparent)]
    InvalidRequest(anyhow::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<JobStoreError> for ApiError {
    fn from(error: JobStoreError) -> Self {
        match error {
            JobStoreError::CapacityExceeded => ApiError::CapacityExceeded,
            JobStoreError::Database(err) => {
                tracing::error!(error = %err, "Database error");
                ApiError::Internal(anyhow::anyhow!(err))
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Internal(err) => {
                tracing::error!(
                    error = %err,
                    "internal server error"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: String::from("internal server error"),
                    }),
                )
                    .into_response()
            }
            ApiError::InvalidJson => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: String::from("invalid_json"),
                }),
            )
                .into_response(),
            ApiError::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ErrorResponse {
                    error: String::from("payload_too_large"),
                }),
            )
                .into_response(),
            ApiError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    error: String::from("too_many_requests"),
                }),
            )
                .into_response(),
            ApiError::QueueClosed => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ErrorResponse {
                    error: String::from("QueueClosed"),
                }),
            )
                .into_response(),
            ApiError::JobNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: String::from("JobNotFound"),
                }),
            )
                .into_response(),
            ApiError::CapacityExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, "1")],
                "job capacity exceeded",
            )
                .into_response(),
            ApiError::JobStore(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: String::from("database error"),
                }),
            )
                .into_response(),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: String::from("unauthorized"),
                }),
            )
                .into_response(),
            ApiError::GithubRequest(err) => {
                tracing::error!(
                    error = %err,
                    "github request failed"
                );
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ErrorResponse {
                        error: String::from("github request failed"),
                    }),
                )
                    .into_response()
            }
            ApiError::DataBase(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: String::from("database error"),
                }),
            )
                .into_response(),
            ApiError::InvalidRequest(err) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: String::from(err.to_string()),
                }),
            )
                .into_response(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e)
    }
}
