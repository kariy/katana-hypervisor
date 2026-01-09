use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use katana_core::HypervisorError;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            ApiError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message
            }
        });

        (status, Json(body)).into_response()
    }
}

impl From<HypervisorError> for ApiError {
    fn from(err: HypervisorError) -> Self {
        match err {
            HypervisorError::InstanceNotFound(name) => {
                ApiError::NotFound(format!("Instance '{}' not found", name))
            }
            HypervisorError::InstanceAlreadyExists(name) => {
                ApiError::Conflict(format!("Instance '{}' already exists", name))
            }
            HypervisorError::InvalidStateTransition { from, to } => ApiError::BadRequest(
                format!("Invalid state transition from {:?} to {:?}", from, to),
            ),
            HypervisorError::PortUnavailable(port) => {
                ApiError::Conflict(format!("Port {} is not available", port))
            }
            HypervisorError::NoPortsAvailable => {
                ApiError::Conflict("No ports available".to_string())
            }
            HypervisorError::StorageQuotaExceeded { used, limit } => ApiError::BadRequest(
                format!("Storage quota exceeded: used {}, limit {}", used, limit),
            ),
            HypervisorError::QemuFailed(msg) => {
                ApiError::Internal(format!("QEMU operation failed: {}", msg))
            }
            HypervisorError::VmProcessNotFound(id) => {
                ApiError::NotFound(format!("VM process not found for instance '{}'", id))
            }
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
