use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// API 层统一错误类型
///
/// 通过 `IntoResponse` 自动将错误转换为带 JSON body 的 HTTP 响应，
/// 前端可以统一解析 `{ "error": "..." }` 格式。
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ApiError {
    /// 底层 IO 错误，根据 ErrorKind 映射到不同 HTTP 状态码
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 路径或资源不存在 → 404
    #[error("Path not found: {0}")]
    NotFound(String),

    /// 路径格式非法或存在目录穿越攻击 → 400
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// 权限不足 → 403
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// 目标已存在（创建/复制/重命名冲突）→ 409
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// 其他未分类错误 → 500
    #[error("{0}")]
    Other(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),

            ApiError::InvalidPath(msg) => (StatusCode::BAD_REQUEST, msg.clone()),

            ApiError::PermissionDenied(msg) => (StatusCode::FORBIDDEN, msg.clone()),

            ApiError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg.clone()),

            // 将 std::io::ErrorKind 映射到语义化的 HTTP 状态码
            ApiError::Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, e.to_string()),
                std::io::ErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, e.to_string()),
                std::io::ErrorKind::AlreadyExists => (StatusCode::CONFLICT, e.to_string()),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            },

            ApiError::Other(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        tracing::warn!(status = status.as_u16(), error = %message, "API error");

        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// 便捷 Result 别名，减少函数签名噪音
pub type ApiResult<T> = Result<T, ApiError>;
