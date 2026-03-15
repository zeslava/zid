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
    #[serde(default)]
    pub csrf_token: Option<String>,
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
    #[serde(default)]
    pub csrf_token: Option<String>,
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
    #[serde(default)]
    pub csrf_token: Option<String>,
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

// --- OIDC/OAuth 2.0 ---

/// Ответ discovery (/.well-known/openid-configuration)
#[derive(Debug, Serialize)]
pub struct OidcDiscoveryResponse {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
}

/// Успешный ответ token endpoint (RFC 6749)
#[derive(Debug, Serialize)]
pub struct TokenSuccessResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Ошибка token endpoint (RFC 6749)
#[derive(Debug, Serialize)]
pub struct TokenErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}
