//! HTTP-тесты OAuth-флоу: логин с `return_to=/oauth/authorize?...`.
//!
//! Регрессия: относительный `return_to`, который выдаёт `oidc_authorize` при истёкшей
//! SSO-сессии, отклонялся `validate_return_to` → 401 вместо возврата в приложение.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tower::ServiceExt;

use crate::adapters::http::handlers::RouterState;
use crate::adapters::http::routes::create_router;
use crate::adapters::http::sso_cookie::ZID_SSO_COOKIE_NAME;
use crate::adapters::persistence::{
    sqlite_credentials::SqliteCredentialsRepository, sqlite_session::SqliteSessionRepository,
    sqlite_ticket::SqliteTicketRepository, sqlite_user::SqliteUserRepository,
};
use crate::application::zid_app::ZidApp;
use crate::ports::credentials_repository::CredentialsRepository;
use crate::ports::user_repository::UserRepository;
use crate::ports::zid_service::ZidService;

const RETURN_TO: &str = "/oauth/authorize?response_type=code&client_id=cfgy&redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fauth%2Fzid%2Fcallback&state=abc";
const CSRF: &str = "test-csrf-token";

struct Fixture {
    router: axum::Router,
    zid: Arc<dyn ZidService>,
}

fn setup() -> Fixture {
    let manager = SqliteConnectionManager::memory()
        .with_init(|c| c.execute_batch("PRAGMA foreign_keys = ON;"));
    let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager).unwrap();

    let users = SqliteUserRepository::new(pool.clone());
    users.create_table().unwrap();
    let credentials = SqliteCredentialsRepository::new(pool.clone());
    credentials.create_table().unwrap();
    let sessions = SqliteSessionRepository::new(pool.clone());
    sessions.create_table().unwrap();
    let tickets = SqliteTicketRepository::new(pool.clone());
    tickets.create_table().unwrap();

    users.create("alice").unwrap();
    credentials.create_user("alice", "secret123").unwrap();

    let zid: Arc<dyn ZidService> = Arc::new(ZidApp::new(
        Arc::new(users),
        Arc::new(sessions),
        Arc::new(credentials),
        Arc::new(tickets),
    ));

    Fixture {
        router: create_router(RouterState::new(zid.clone())),
        zid,
    }
}

fn form_post(uri: &str, body: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, format!("zid_csrf={CSRF}"))
        .body(Body::from(body))
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Основной сценарий бага: истёкшая сессия → форма логина с относительным
/// `return_to=/oauth/authorize?...` → вход должен возвращать в OAuth-флоу, а не в 401.
#[tokio::test]
async fn login_with_oauth_return_to_redirects_back_to_authorize() {
    let f = setup();

    let body = format!(
        "username=alice&password=secret123&csrf_token={CSRF}&return_to={}",
        urlencoding::encode(RETURN_TO)
    );
    let resp = f.router.oneshot(form_post("/", body)).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .expect("SSO cookie должна выставляться")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with(&format!("{ZID_SSO_COOKIE_NAME}=")));

    let html = body_string(resp).await;
    // Для /oauth/authorize редирект идёт как есть, без ticket
    assert!(html.contains("/oauth/authorize?response_type=code"), "{html}");
    assert!(!html.contains("ticket="), "{html}");
}

/// Ветка "Continue as ..." — тот же относительный return_to.
#[tokio::test]
async fn continue_as_with_oauth_return_to_redirects_back_to_authorize() {
    let f = setup();

    let ticket = f.zid.login("alice", "secret123", None).unwrap();
    let session_id = ticket.session_id.clone();

    let req = Request::builder()
        .method("POST")
        .uri("/continue")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(
            header::COOKIE,
            format!("zid_csrf={CSRF}; {ZID_SSO_COOKIE_NAME}={session_id}"),
        )
        .body(Body::from(format!(
            "csrf_token={CSRF}&return_to={}",
            urlencoding::encode(RETURN_TO)
        )))
        .unwrap();

    let resp = f.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = body_string(resp).await;
    assert!(html.contains("/oauth/authorize?response_type=code"), "{html}");
}

/// Чужой origin в return_to по-прежнему отклоняется.
#[tokio::test]
async fn login_with_untrusted_return_to_is_rejected() {
    let f = setup();

    let body = format!(
        "username=alice&password=secret123&csrf_token={CSRF}&return_to={}",
        urlencoding::encode("https://evil.com/steal")
    );
    let resp = f.router.oneshot(form_post("/", body)).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Карточка "уже вошли" должна называть текущего пользователя (account chooser UX).
#[tokio::test]
async fn login_form_shows_current_username() {
    let f = setup();

    let ticket = f.zid.login("alice", "secret123", None).unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/")
        .header(
            header::COOKIE,
            format!("{ZID_SSO_COOKIE_NAME}={}", ticket.session_id),
        )
        .body(Body::empty())
        .unwrap();

    let resp = f.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let html = body_string(resp).await;
    assert!(html.contains("Signed in as <strong>alice</strong>"), "{html}");
    assert!(html.contains("Continue as alice"), "{html}");
}

/// Протухшая/несуществующая сессия — обычная форма логина и очистка cookie.
#[tokio::test]
async fn login_form_with_dead_session_shows_form() {
    let f = setup();

    let req = Request::builder()
        .method("GET")
        .uri("/")
        .header(
            header::COOKIE,
            format!("{ZID_SSO_COOKIE_NAME}=00000000-0000-0000-0000-000000000000"),
        )
        .body(Body::empty())
        .unwrap();

    let resp = f.router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let cleared = resp
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .any(|v| {
            let v = v.to_str().unwrap();
            v.starts_with(&format!("{ZID_SSO_COOKIE_NAME}=")) && v.contains("Max-Age=0")
        });
    assert!(cleared, "протухшая SSO cookie должна очищаться");

    let html = body_string(resp).await;
    assert!(html.contains("name=\"password\""), "{html}");
    assert!(!html.contains("Signed in as"), "{html}");
}

/// Страница ошибки не теряет return_to — пользователь остаётся в OAuth-флоу.
#[tokio::test]
async fn failed_login_keeps_return_to_in_back_link() {
    let f = setup();

    let body = format!(
        "username=alice&password=wrong&csrf_token={CSRF}&return_to={}",
        urlencoding::encode(RETURN_TO)
    );
    let resp = f.router.oneshot(form_post("/", body)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let html = body_string(resp).await;
    assert!(html.contains("return_to=%2Foauth%2Fauthorize"), "{html}");
}
