# TODO — Open-Source Production Readiness

## Blockers

- [x] **XSS в return_to** — `return_to` вставлялся в HTML/JS без экранирования (`handlers.rs`). Исправлено: добавлена `html_escape()`.
- [ ] **.env в git history** — файл содержит реальные пароли (`POSTGRES_PASSWORD=12345671`, хост `opi5`). Нужно вычистить через `git filter-repo` или `BFG`.
- [ ] **Rate limiting** — brute-force возможен на `/login`, `/register`, `/oauth/token`. Добавить `tower-governor` или аналог.
- [ ] **unwrap() в не-тестовом коде** — ~30 вызовов. Заменить на `?` / `.expect("reason")` с обоснованием.

## Important

- [ ] **Structured logging** — заменить `println!/eprintln!` на `tracing`. Особенно в `postgres_credentials.rs`, `redis_credentials.rs`, `zid_app.rs`.
- [ ] **Input validation** — нет ограничений на длину username/password в DTO. Добавить min/max length.
- [ ] **CSRF токены** — HTML-формы (`POST /`, `POST /register`, `POST /continue`) не защищены от CSRF.
- [ ] **Cargo.toml metadata** — добавить `description`, `license = "Apache-2.0"`, `repository`.

## Nice to Have

- [ ] **CORS middleware** — добавить `tower-http::CorsLayer` если планируется кросс-доменный API-доступ из браузеров.
- [ ] **SqliteAuthCodeRepository** — OIDC auth codes работают только с Redis. Для полной SQLite-поддержки нужен SQLite-адаптер.
- [ ] **Account lockout** — блокировка после N неудачных попыток входа.
- [ ] **Request ID / tracing spans** — для отладки в production (после перехода на `tracing`).
