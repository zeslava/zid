// HTTP handlers (login, verify, logout)

use std::sync::Arc;

use axum::{
    Form, Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    adapters::http::dto::{
        CreateUserRequest, CreateUserResponse, LoginRequest, LoginResponse, LogoutRequest,
        LogoutResponse, VerifyRequest, VerifyResponse,
    },
    ports::{error::Error, zid_service::ZidService},
};

#[derive(Clone)]
pub struct RouterState {
    pub zid: Arc<dyn ZidService>,
}

impl RouterState {
    pub fn new(zid: Arc<dyn ZidService>) -> Self {
        Self { zid }
    }
}

// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

pub async fn login_form(State(_state): State<RouterState>) -> impl IntoResponse {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>ZID Login</title>
            <style>
                body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }
                form { display: flex; flex-direction: column; gap: 10px; }
                input { padding: 10px; border: 1px solid #ddd; border-radius: 4px; }
                button { padding: 10px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }
                button:hover { background: #0056b3; }
                .success { color: green; padding: 10px; background: #d4edda; border-radius: 4px; margin-bottom: 10px; }
                .error { color: red; padding: 10px; background: #f8d7da; border-radius: 4px; margin-bottom: 10px; }
            </style>
        </head>
        <body>
            <h1>ZID CAS Login</h1>
            <form method="post" action="/">
                <input type="text" name="username" placeholder="Username" required />
                <input type="password" name="password" placeholder="Password" required />
                <input type="text" name="return_to" placeholder="Return URL" value="http://localhost:3000" required />
                <button type="submit">Login</button>
            </form>
        </body>
        </html>
    "#;

    axum::response::Html(html)
}

pub async fn login_form_submit(
    State(state): State<RouterState>,
    Form(req): Form<LoginRequest>,
) -> impl IntoResponse {
    let return_to = req.return_to.clone();

    // Запускаем синхронный код в отдельном thread pool
    let result = tokio::task::spawn_blocking(move || {
        state
            .zid
            .login(&req.username, &req.password, &req.return_to)
    })
    .await;

    // Handle the result and return appropriate HTML
    match result {
        Ok(Ok(ticket)) => {
            let redirect_url = format!("{}?ticket={}", return_to, ticket.id);

            // Возвращаем HTML с редиректом
            let html = format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Redirecting</title>
                    <meta http-equiv="refresh" content="0;url={}" />
                    <style>
                        body {{ font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }}
                        .message {{ padding: 20px; background: #d4edda; border-radius: 4px; text-align: center; }}
                    </style>
                </head>
                <body>
                    <div class="message">
                        <p>Redirecting...</p>
                        <p>If not redirected, <a href="{}">click here</a></p>
                    </div>
                </body>
                </html>
                "#,
                redirect_url, redirect_url
            );

            axum::response::Html(html).into_response()
        }
        Ok(Err(_e)) => {
            // Log the error for debugging (only on server)
            // Error details already logged in business layer

            // Return minimal error page
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Unauthorized</title>
                    <style>
                        body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }
                        .error { padding: 20px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; text-align: center; }
                        a { color: #007bff; text-decoration: none; }
                        a:hover { text-decoration: underline; }
                    </style>
                </head>
                <body>
                    <div class="error">
                        <h2>Unauthorized</h2>
                        <p><a href="/">← Back</a></p>
                    </div>
                </body>
                </html>
            "#;

            (StatusCode::UNAUTHORIZED, axum::response::Html(html)).into_response()
        }
        Err(_e) => {
            // Log the error for debugging (only on server)
            // Task error already logged

            // Return minimal error page
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Unauthorized</title>
                    <style>
                        body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }
                        .error { padding: 20px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; text-align: center; }
                        a { color: #007bff; text-decoration: none; }
                        a:hover { text-decoration: underline; }
                    </style>
                </head>
                <body>
                    <div class="error">
                        <h2>Unauthorized</h2>
                        <p><a href="/">← Back</a></p>
                    </div>
                </body>
                </html>
            "#;

            (StatusCode::UNAUTHORIZED, axum::response::Html(html)).into_response()
        }
    }
}

pub async fn login_json(
    State(state): State<RouterState>,
    Json(req): Json<LoginRequest>,
) -> Result<LoginResponse, HttpError> {
    let return_to = req.return_to.clone();

    // Запускаем синхронный код в отдельном thread pool
    let ticket = tokio::task::spawn_blocking(move || {
        state
            .zid
            .login(&req.username, &req.password, &req.return_to)
    })
    .await??;

    let redirect_url = format!("{}?ticket={}", return_to, ticket.id);

    Ok(LoginResponse {
        ticket: ticket.id,
        redirect_url,
    })
}

pub async fn verify(
    State(state): State<RouterState>,
    Json(req): Json<VerifyRequest>,
) -> Result<VerifyResponse, HttpError> {
    // Запускаем синхронный код в отдельном thread pool
    let result =
        tokio::task::spawn_blocking(move || state.zid.verify(&req.ticket, &req.service)).await??;

    Ok(VerifyResponse {
        success: true,
        user_id: result.user_id,
        username: result.username,
        session_id: result.session_id,
    })
}

pub async fn logout(
    State(state): State<RouterState>,
    Json(cmd): Json<LogoutRequest>,
) -> Result<LogoutResponse, HttpError> {
    // Запускаем синхронный код в отдельном thread pool
    tokio::task::spawn_blocking(move || state.zid.logout(&cmd.session_id)).await??;

    Ok(LogoutResponse {})
}

pub async fn create_user(
    State(state): State<RouterState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<CreateUserResponse, HttpError> {
    let username = req.username.clone();

    // Запускаем синхронный код в отдельном thread pool
    tokio::task::spawn_blocking(move || state.zid.create_user(&req.username, &req.password))
        .await??;

    Ok(CreateUserResponse {
        success: true,
        username,
    })
}

// HTTP error wrapper type
pub struct HttpError(crate::ports::error::Error);

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        // Log the actual error for debugging (only on server)
        eprintln!("Domain error: {:?}", self.0);

        // Map domain errors to HTTP status codes
        let status = match self.0 {
            Error::AuthenticationFailed | Error::UserNotFound | Error::InvalidCredentials => {
                StatusCode::UNAUTHORIZED
            }

            Error::TicketNotFound
            | Error::TicketExpired
            | Error::TicketConsumed
            | Error::ServiceMismatch { .. }
            | Error::SessionNotFound => StatusCode::FORBIDDEN,

            Error::UserAlreadyExists => StatusCode::CONFLICT,

            Error::RepositoryError(_) | Error::InternalError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        // Return only status code, no error details
        status.into_response()
    }
}

impl From<crate::ports::error::Error> for HttpError {
    fn from(err: crate::ports::error::Error) -> Self {
        Self(err)
    }
}

// Convert tokio JoinError to Error
impl From<tokio::task::JoinError> for HttpError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self(crate::ports::error::Error::InternalError(err.to_string()))
    }
}
