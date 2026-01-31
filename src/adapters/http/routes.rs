// Маршруты
use crate::adapters::http::handlers::*;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static"]
pub struct StaticAssets;

// Handler для обслуживания встроенных файлов
pub async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match StaticAssets::get(path.as_str()) {
        Some(content) => {
            let mime_type = if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".js") {
                "application/javascript"
            } else {
                "application/octet-stream"
            };

            (
                axum::http::StatusCode::OK,
                [("Content-Type", mime_type)],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

pub fn create_router(state: RouterState) -> axum::Router {
    let mut router = axum::Router::new()
        .route("/health", get(health_check))
        .route("/", get(login_form).post(login_form_submit))
        .route("/continue", post(continue_as_form_submit))
        .route("/register", get(register_form).post(register_form_submit))
        .route("/login", post(login_json))
        .route("/login/telegram", post(login_telegram))
        .route("/verify", post(verify))
        .route("/logout", post(logout))
        .route("/static/{*path}", get(serve_static));

    if state.oidc.is_some() {
        router = router
            .route("/.well-known/openid-configuration", get(oidc_discovery))
            .route("/oauth/authorize", get(oidc_authorize))
            .route("/oauth/token", post(oidc_token))
            .route("/oauth/userinfo", get(oidc_userinfo))
            .route("/oauth/jwks", get(oidc_jwks));
    }

    router.with_state(state)
}
