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
    pub return_to: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginResponse {
    pub ticket: String,
    pub redirect_url: String,
}

impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserResponse {
    pub success: bool,
    pub username: String,
}

impl IntoResponse for CreateUserResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}
