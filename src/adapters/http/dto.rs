// DTO для HTTP запросов/ответов

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub ticket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Запрос на "Continue as ..." по существующей SSO-сессии ZID
///
/// Клиент передаёт `return_to` (опционально) и использует cookie `zid_sso`
/// для аутентификации на стороне ZID.
#[derive(Debug, Deserialize, Serialize)]
pub struct ContinueAsRequest {
    #[serde(default)]
    pub return_to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyRequest {
    pub ticket: String, // ZID-{32_символа} (пример: ZID-7a3b9c2f8e1d4a5b6c7d8e9f0a1b2c3d)
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub user_id: String,
    pub username: String,
    pub session_id: String,
}

impl IntoResponse for VerifyResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogoutRequest {
    pub session_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogoutResponse {}

impl IntoResponse for LogoutResponse {
    fn into_response(self) -> Response {
        StatusCode::OK.into_response()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub password_confirm: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TelegramLoginRequest {
    pub id: i64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
    #[serde(default)]
    pub return_to: Option<String>,
}
