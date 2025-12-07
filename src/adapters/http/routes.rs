// Маршруты
use crate::adapters::http::handlers::*;
use axum::routing::{get, post};

pub fn create_router(state: RouterState) -> axum::Router {
    axum::Router::new()
        .route("/health", get(health_check))
        .route("/", get(login_form).post(login_form_submit))
        .route("/login", post(login_json))
        .route("/verify", post(verify))
        .route("/logout", post(logout))
        .route("/register", post(create_user))
        .with_state(state)
}
