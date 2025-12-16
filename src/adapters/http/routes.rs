// Маршруты
use crate::adapters::http::handlers::*;
use axum::routing::{get, post};

pub fn create_router(state: RouterState) -> axum::Router {
    axum::Router::new()
        .route("/health", get(health_check))
        .route("/", get(login_form).post(login_form_submit))
        .route("/continue", post(continue_as_form_submit))
        .route("/register", get(register_form).post(register_form_submit))
        .route("/login", post(login_json))
        .route("/login/telegram", post(login_telegram))
        .route("/verify", post(verify))
        .route("/logout", post(logout))
        .with_state(state)
}
