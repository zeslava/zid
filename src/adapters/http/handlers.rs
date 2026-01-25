// HTTP handlers (login, verify, logout)

use std::sync::Arc;

use axum::debug_handler;
use axum::{
    Form, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{
    adapters::http::{
        dto::{
            ContinueAsRequest, CreateUserRequest, LoginRequest, LoginResponse, LogoutRequest,
            LogoutResponse, TelegramLoginRequest, VerifyRequest, VerifyResponse,
        },
        sso_cookie::{
            DEFAULT_SSO_TTL_SECS, build_clear_cookie, build_set_cookie, default_config_for_request,
            get_sso_session_id,
        },
    },
    adapters::telegram::TelegramAuthData,
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

pub async fn login_form(
    State(state): State<RouterState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let return_to = params.get("return_to").map(|s| s.as_str()).unwrap_or("");

    // Если есть SSO cookie — аккуратно проверяем, что сессия действительно валидна.
    //
    // Если сессия невалидна/просрочена:
    // - не показываем "Continue"
    // - чистим cookie на клиенте (чтобы не было "залипания" на протухшей сессии)
    // - показываем обычную форму логина
    //
    // Важно: никаких повторных проверок — решение принимается один раз, далее просто строим ответ.
    let (show_continue, clear_cookie): (bool, bool) = match get_sso_session_id(&headers) {
        None => (false, false),
        Some(session_id) => {
            let state2 = state.clone();
            let session_id2 = session_id.clone();

            let valid = tokio::task::spawn_blocking(move || {
                // continue_as() делает:
                // - sessions.get(session_id) (валидирует/чистит протухшую на стороне хранилища),
                // - refresh expiry (sliding) и выдает тикет.
                //
                // Здесь нам тикет не нужен — но это единственный публичный сервисный метод,
                // гарантирующий проверку валидности session_id без прямого доступа к репозиториям.
                state2.zid.continue_as(&session_id2, None).map(|_| ())
            })
            .await;

            match valid {
                Ok(Ok(())) => (true, false),
                _ => (false, true),
            }
        }
    };

    if show_continue {
        let html = format!(
            r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <title>ZID Login</title>
                        <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
                        <style>
                            body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 100px auto; padding: 20px; }}
                            .card {{ padding: 20px; border: 1px solid #ddd; border-radius: 8px; }}
                            .muted {{ color: #666; }}
                            .row {{ display: flex; gap: 10px; margin-top: 15px; }}
                            button {{ padding: 10px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }}
                            button:hover {{ background: #0056b3; }}
                            form {{ margin: 0; }}
                            .secondary {{ background: #6c757d; }}
                            .secondary:hover {{ background: #545b62; }}
                            a {{ color: #007bff; text-decoration: none; }}
                            a:hover {{ text-decoration: underline; }}
                        </style>
                    </head>
                    <body>
                        <h1>ZID CAS Login</h1>
                        <div class="card">
                            <p class="muted">You're already signed in.</p>

                            <div class="row">
                                <form method="post" action="/continue">
                                    <input type="hidden" name="return_to" value="{}" />
                                    <button type="submit">Continue</button>
                                </form>

                                <form method="get" action="/">
                                    <button type="submit" class="secondary">Sign in as another user</button>
                                </form>
                            </div>

                            <p class="muted" style="margin-top: 15px;">
                                If you want to sign out from ZID, use the API <code>/logout</code>.
                            </p>
                        </div>
                    </body>
                    </html>
                    "#,
            return_to
        );

        return axum::response::Html(html).into_response();
    }

    // Получаем bot username для Telegram Widget (опционально)
    let telegram_bot_username = std::env::var("TELEGRAM_BOT_USERNAME").unwrap_or_default();

    // Telegram Widget будет показываться только если настроен bot username
    let telegram_widget = if !telegram_bot_username.is_empty() {
        format!(
            r#"
            <div class="divider">
                <span>OR</span>
            </div>
            <div id="telegram-login-container">
                <script async src="https://telegram.org/js/telegram-widget.js?22"
                    data-telegram-login="{}"
                    data-size="large"
                    data-onauth="onTelegramAuth(user)"
                    data-request-access="write">
                </script>
            </div>
            "#,
            telegram_bot_username
        )
    } else {
        String::new()
    };

    let html = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>ZID Login</title>
            <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
            <style>
                body {{ font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }}
                form {{ display: flex; flex-direction: column; gap: 10px; }}
                input {{ padding: 10px; border: 1px solid #ddd; border-radius: 4px; }}
                button {{ padding: 10px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; }}
                button:hover {{ background: #0056b3; }}
                .link {{ text-align: center; margin-top: 15px; }}
                a {{ color: #007bff; text-decoration: none; }}
                a:hover {{ text-decoration: underline; }}
                .divider {{
                    display: flex;
                    align-items: center;
                    text-align: center;
                    margin: 20px 0;
                }}
                .divider::before,
                .divider::after {{
                    content: '';
                    flex: 1;
                    border-bottom: 1px solid #ddd;
                }}
                .divider span {{
                    padding: 0 10px;
                    color: #666;
                    font-size: 14px;
                }}
                #telegram-login-container {{
                    display: flex;
                    justify-content: center;
                    margin: 15px 0;
                }}
                .loading {{
                    text-align: center;
                    color: #666;
                    padding: 10px;
                }}
            </style>
        </head>
        <body>
            <h1>ZID CAS Login</h1>
            <form method="post" action="/">
                <input type="text" name="username" placeholder="Username" required />
                <input type="password" name="password" placeholder="Password" required />
                <input type="hidden" name="return_to" value="{}" />
                <button type="submit">Login</button>
            </form>
            {}
            <div class="link">
                <a href="/register">Don't have an account? Register</a>
            </div>
            <script>
                // Обработчик успешной аутентификации через Telegram
                function onTelegramAuth(user) {{
                    console.log('Telegram auth success:', user);

                    // Показываем индикатор загрузки
                    const container = document.getElementById('telegram-login-container');
                    container.innerHTML = '<div class="loading">Logging in via Telegram...</div>';

                    // Отправляем данные на сервер
                    fetch('/login/telegram', {{
                        method: 'POST',
                        headers: {{
                            'Content-Type': 'application/json',
                        }},
                        body: JSON.stringify({{
                            id: user.id,
                            first_name: user.first_name,
                            last_name: user.last_name,
                            username: user.username,
                            photo_url: user.photo_url,
                            auth_date: user.auth_date,
                            hash: user.hash,
                            return_to: '{}'
                        }})
                    }})
                    .then(response => {{
                        if (!response.ok) {{
                            return response.json().then(err => {{
                                throw new Error(err.error || 'Authentication failed');
                            }});
                        }}
                        return response.json();
                    }})
                    .then(data => {{
                        console.log('Login successful:', data);
                        // Редирект на return_to с тикетом
                        window.location.href = data.redirect_url;
                    }})
                    .catch(error => {{
                        console.error('Error:', error);
                        container.innerHTML = '<div class="loading" style="color: #dc3545;">Error: ' + error.message + '</div>';
                        // Восстанавливаем Telegram widget через 3 секунды
                        setTimeout(() => {{
                            location.reload();
                        }}, 3000);
                    }});
                }}
            </script>
        </body>
        </html>
        "#,
        return_to, telegram_widget, return_to
    );

    if clear_cookie {
        let mut cookie_cfg = default_config_for_request(&headers);
        cookie_cfg.ttl_secs = DEFAULT_SSO_TTL_SECS;
        let clear_cookie_value = build_clear_cookie(&cookie_cfg);

        let mut resp = axum::response::Html(html).into_response();
        resp.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_str(&clear_cookie_value).unwrap(),
        );
        return resp;
    }

    axum::response::Html(html).into_response()
}

pub async fn register_form(State(_state): State<RouterState>) -> impl IntoResponse {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>ZID Registration</title>
            <link rel="icon" type="image/svg+xml" href="/static/favicon.svg">
            <style>
                body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; }
                form { display: flex; flex-direction: column; gap: 10px; }
                input { padding: 10px; border: 1px solid #ddd; border-radius: 4px; }
                input.error { border-color: #dc3545; }
                button { padding: 10px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; }
                button:hover { background: #218838; }
                button:disabled { background: #6c757d; cursor: not-allowed; }
                .link { text-align: center; margin-top: 15px; }
                a { color: #007bff; text-decoration: none; }
                a:hover { text-decoration: underline; }
                .error-message { color: #dc3545; font-size: 14px; margin-top: -5px; display: none; }
                .error-message.show { display: block; }
            </style>
        </head>
        <body>
            <h1>Create Account</h1>
            <form method="post" action="/register" id="registerForm">
                <input type="text" name="username" id="username" placeholder="Username" required minlength="3" />
                <input type="password" name="password" id="password" placeholder="Password" required minlength="6" />
                <input type="password" name="password_confirm" id="password_confirm" placeholder="Confirm Password" required minlength="6" />
                <div class="error-message" id="passwordError">Passwords do not match</div>
                <button type="submit" id="submitBtn">Register</button>
            </form>
            <div class="link">
                <a href="/">Already have an account? Login</a>
            </div>
            <script>
                const password = document.getElementById('password');
                const passwordConfirm = document.getElementById('password_confirm');
                const submitBtn = document.getElementById('submitBtn');
                const errorMessage = document.getElementById('passwordError');

                function validatePasswords() {
                    if (passwordConfirm.value.length > 0) {
                        if (password.value !== passwordConfirm.value) {
                            passwordConfirm.classList.add('error');
                            errorMessage.classList.add('show');
                            submitBtn.disabled = true;
                        } else {
                            passwordConfirm.classList.remove('error');
                            errorMessage.classList.remove('show');
                            submitBtn.disabled = false;
                        }
                    } else {
                        passwordConfirm.classList.remove('error');
                        errorMessage.classList.remove('show');
                        submitBtn.disabled = false;
                    }
                }

                password.addEventListener('input', validatePasswords);
                passwordConfirm.addEventListener('input', validatePasswords);
            </script>
        </body>
        </html>
    "#;

    axum::response::Html(html)
}

#[debug_handler]
pub async fn login_form_submit(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Form(req): Form<LoginRequest>,
) -> impl IntoResponse {
    let return_to = req.return_to.clone();
    let username = req.username.clone();

    // Запускаем синхронный код в отдельном thread pool
    let result = tokio::task::spawn_blocking(move || {
        state
            .zid
            .login(&req.username, &req.password, req.return_to.as_deref())
    })
    .await;

    // Handle the result and return appropriate HTML
    match result {
        Ok(Ok(ticket)) => {
            // ZID SSO cookie should store the *session id*.
            // A ticket is created for a session, so we must use `ticket.session_id` here.
            // Refresh/issue SSO cookie (sliding expiration on client).
            //
            // Secure-by-default for production:
            // - if running behind HTTPS + proxy provides `X-Forwarded-Proto=https` -> Secure=true
            // - for local dev over plain HTTP: set `ZID_COOKIE_SECURE=false`
            let mut cookie_cfg = default_config_for_request(&headers);
            cookie_cfg.ttl_secs = DEFAULT_SSO_TTL_SECS;
            let set_cookie_value = build_set_cookie(&ticket.session_id, &cookie_cfg);

            let _existing = get_sso_session_id(&headers);

            // Check if return_to is provided and not empty
            if let Some(ref url) = return_to {
                if !url.is_empty() {
                    let redirect_url = format!("{}?ticket={}", url, ticket.id);

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

                    let mut resp = axum::response::Html(html).into_response();
                    resp.headers_mut().insert(
                        axum::http::header::SET_COOKIE,
                        axum::http::HeaderValue::from_str(&set_cookie_value).unwrap(),
                    );
                    return resp;
                }
            }

            // No redirect - return success page with ticket info
            let html = format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Login Successful</title>
                    <style>
                        body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 100px auto; padding: 20px; }}
                        .success {{ padding: 20px; background: #d4edda; border: 1px solid #c3e6cb; border-radius: 4px; text-align: center; }}
                        .ticket {{ font-family: monospace; background: #f8f9fa; padding: 10px; border-radius: 4px; margin: 15px 0; word-break: break-all; }}
                        h2 {{ color: #155724; }}
                    </style>
                </head>
                <body>
                    <div class="success">
                        <h2>✓ Login Successful!</h2>
                        <p>Your authentication ticket:</p>
                        <div class="ticket">{}</div>
                    </div>
                </body>
                </html>
                "#,
                ticket.id
            );

            let mut resp = axum::response::Html(html).into_response();
            resp.headers_mut().insert(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&set_cookie_value).unwrap(),
            );
            resp
        }
        Ok(Err(e)) => {
            // Log the error for debugging (only on server)
            eprintln!("Login failed for user '{}': {:?}", username, e);

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

/// "Continue as ..." submit handler.
///
/// Uses existing ZID SSO cookie (`zid_sso`) to:
/// 1) validate the session,
/// 2) refresh session expiry (sliding expiration),
/// 3) issue a new one-time ticket for the provided `return_to`,
/// 4) refresh the browser cookie (sliding expiration on client as well),
/// 5) redirect if `return_to` is provided, otherwise return a success page with the ticket.
#[debug_handler]
pub async fn continue_as_form_submit(
    State(state): State<RouterState>,
    headers: HeaderMap,
    Form(req): Form<ContinueAsRequest>,
) -> impl IntoResponse {
    let session_id = match get_sso_session_id(&headers) {
        Some(s) => s,
        None => {
            // No cookie -> user must log in again
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Unauthorized</title>
                </head>
                <body>
                    <h2>Unauthorized</h2>
                    <p><a href="/">Back to login</a></p>
                </body>
                </html>
            "#;
            return (StatusCode::UNAUTHORIZED, axum::response::Html(html)).into_response();
        }
    };
    let return_to = req.return_to.clone();

    // Avoid moves across await boundaries by cloning what we need
    let session_id_for_service = session_id.clone();
    let return_to_for_service = return_to.clone();

    let result = tokio::task::spawn_blocking(move || {
        state
            .zid
            .continue_as(&session_id_for_service, return_to_for_service.as_deref())
    })
    .await;

    match result {
        Ok(Ok(ticket)) => {
            // Refresh cookie too (client sliding expiration)
            //
            // Secure-by-default for production:
            // - if running behind HTTPS + proxy provides `X-Forwarded-Proto=https` -> Secure=true
            // - for local dev over plain HTTP: set `ZID_COOKIE_SECURE=false`
            let mut cookie_cfg = default_config_for_request(&headers);
            cookie_cfg.ttl_secs = DEFAULT_SSO_TTL_SECS;
            let set_cookie_value = build_set_cookie(&session_id, &cookie_cfg);

            if let Some(url) = return_to.as_ref().filter(|s| !s.is_empty()) {
                let redirect_url = format!("{}?ticket={}", url, ticket.id);

                let html = format!(
                    r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <title>Redirecting</title>
                        <meta http-equiv="refresh" content="0;url={}" />
                    </head>
                    <body>
                        <p>Redirecting... <a href="{}">click here</a></p>
                    </body>
                    </html>
                    "#,
                    redirect_url, redirect_url
                );

                let mut resp = axum::response::Html(html).into_response();
                resp.headers_mut().insert(
                    axum::http::header::SET_COOKIE,
                    axum::http::HeaderValue::from_str(&set_cookie_value).unwrap(),
                );
                return resp;
            }

            // No return_to -> show ticket
            let html = format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Ticket Issued</title>
                    <style>
                        body {{ font-family: Arial, sans-serif; max-width: 500px; margin: 100px auto; padding: 20px; }}
                        .success {{ padding: 20px; background: #d4edda; border: 1px solid #c3e6cb; border-radius: 4px; text-align: center; }}
                        .ticket {{ font-family: monospace; background: #f8f9fa; padding: 10px; border-radius: 4px; margin: 15px 0; word-break: break-all; }}
                    </style>
                </head>
                <body>
                    <div class="success">
                        <h2>✓ Ticket issued</h2>
                        <p>Your authentication ticket:</p>
                        <div class="ticket">{}</div>
                    </div>
                </body>
                </html>
                "#,
                ticket.id
            );

            let mut resp = axum::response::Html(html).into_response();
            resp.headers_mut().insert(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&set_cookie_value).unwrap(),
            );
            resp
        }
        Ok(Err(e)) => {
            eprintln!("Continue-as failed: {:?}", e);
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Unauthorized</title>
                </head>
                <body>
                    <h2>Unauthorized</h2>
                    <p><a href="/">Back to login</a></p>
                </body>
                </html>
            "#;
            (StatusCode::UNAUTHORIZED, axum::response::Html(html)).into_response()
        }
        Err(_e) => {
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Unauthorized</title>
                </head>
                <body>
                    <h2>Unauthorized</h2>
                    <p><a href="/">Back to login</a></p>
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
            .login(&req.username, &req.password, req.return_to.as_deref())
    })
    .await??;

    // Build redirect_url only if return_to is provided
    let redirect_url = return_to
        .filter(|s| !s.is_empty())
        .map(|url| format!("{}?ticket={}", url, ticket.id));

    Ok(LoginResponse {
        ticket: ticket.id,
        redirect_url,
    })
}

pub async fn login_telegram(
    State(state): State<RouterState>,
    Json(req): Json<TelegramLoginRequest>,
) -> Result<LoginResponse, HttpError> {
    // Получаем токен бота из переменной окружения
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN").map_err(|_| {
        HttpError(Error::InternalError(
            "TELEGRAM_BOT_TOKEN not configured".to_string(),
        ))
    })?;

    // Создаем структуру для валидации
    let auth_data = TelegramAuthData {
        id: req.id,
        first_name: req.first_name.clone(),
        last_name: req.last_name.clone(),
        username: req.username.clone(),
        photo_url: req.photo_url.clone(),
        auth_date: req.auth_date,
        hash: req.hash.clone(),
    };

    // Проверяем подлинность данных от Telegram
    auth_data
        .verify(&bot_token)
        .map_err(|_e| HttpError(Error::AuthenticationFailed))?;

    let return_to = req.return_to.clone();

    // Запускаем синхронный код в отдельном thread pool
    let ticket = tokio::task::spawn_blocking(move || {
        state.zid.login_telegram(
            req.id,
            req.username,
            req.first_name,
            req.last_name,
            req.return_to.as_deref(),
        )
    })
    .await??;

    // Build redirect_url only if return_to is provided
    let redirect_url = return_to
        .filter(|s| !s.is_empty())
        .map(|url| format!("{}?ticket={}", url, ticket.id));

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

pub async fn register_form_submit(
    State(state): State<RouterState>,
    Form(req): Form<CreateUserRequest>,
) -> impl IntoResponse {
    // Проверяем совпадение паролей
    if req.password != req.password_confirm {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Registration Failed</title>
                <style>
                    body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
                    .error { padding: 20px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; }
                    a { color: #007bff; text-decoration: none; margin-top: 10px; display: inline-block; }
                </style>
            </head>
            <body>
                <div class="error">
                    <h2>Registration Failed</h2>
                    <p>Passwords do not match</p>
                    <a href="/register">← Try again</a>
                </div>
            </body>
            </html>
        "#;
        return (StatusCode::BAD_REQUEST, axum::response::Html(html)).into_response();
    }

    // Запускаем синхронный код в отдельном thread pool
    let result =
        tokio::task::spawn_blocking(move || state.zid.create_user(&req.username, &req.password))
            .await;

    match result {
        Ok(Ok(_)) => {
            // Success - redirect to login
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Registration Successful</title>
                    <meta http-equiv="refresh" content="2;url=/" />
                    <style>
                        body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
                        .success { color: #155724; padding: 20px; background: #d4edda; border: 1px solid #c3e6cb; border-radius: 4px; }
                        a { color: #007bff; text-decoration: none; }
                    </style>
                </head>
                <body>
                    <div class="success">
                        <h2>✓ Registration Successful!</h2>
                        <p>Redirecting to login page...</p>
                        <p><a href="/">Click here if not redirected</a></p>
                    </div>
                </body>
                </html>
            "#;
            (StatusCode::OK, axum::response::Html(html)).into_response()
        }
        Ok(Err(_)) | Err(_) => {
            // Error - show error page
            let html = r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Registration Failed</title>
                    <style>
                        body { font-family: Arial, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
                        .error { padding: 20px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; }
                        a { color: #007bff; text-decoration: none; margin-top: 10px; display: inline-block; }
                    </style>
                </head>
                <body>
                    <div class="error">
                        <h2>Registration Failed</h2>
                        <p>Username already exists or invalid input</p>
                        <a href="/register">← Try again</a>
                    </div>
                </body>
                </html>
            "#;
            (StatusCode::CONFLICT, axum::response::Html(html)).into_response()
        }
    }
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
