# ZID — core

Lightweight self-hosted IdP на Rust. Флоу: логин → одноразовый ticket (5 мин) → приложение вызывает `/verify` → получает `user_id`/`username` и заводит свою сессию. SSO-сессия — 7 дней, sliding expiration.

Обязательное чтение в корне репозитория: `AGENTS.md` и `CLAUDE.md` (правила для агентов, список env, HTTP API). Не дублировать их содержимое в памяти.

## Карта исходников (гексагональная архитектура)

- `src/ports/` — доменные трейты и сущности: `zid_service.rs`, `oidc_service.rs`, `client_store.rs`, `*_repository.rs`, `entities.rs` (User/Session/Ticket), `error.rs`.
- `src/application/` — бизнес-логика: `zid_app.rs` (ZidService), `oidc_app.rs` (OidcService), `oidc_jwt.rs` (RS256-подпись, JWKS).
- `src/adapters/http/` — Axum: `routes.rs` (+ rust-embed статики из `static/`), `handlers.rs` (~1000 строк, HTML генерится инлайн в Rust, шаблонизатора нет), `dto.rs`, `sso_cookie.rs`, `flow_tests.rs`.
- `src/adapters/persistence/` — по три реализации на сущность: `postgres_*`, `redis_*`, `sqlite_*` (+ только postgres для `auth_code`).
- `src/adapters/oidc/file_client_store.rs` — клиенты из YAML (`oidc_clients.yaml`).
- `src/adapters/telegram/` — Telegram Login Widget (HMAC-проверка).
- `src/cli.rs` — clap: `zid serve` (по умолчанию) и `zid oidc-client {list,add,...}` с интерактивным inquire.
- `src/migrations.rs` — миграции **вшиты в бинарник** через `include_str!` и применяются на старте (`run_pg`, таблица `_migrations`). При добавлении файла в `migrations/` его нужно вручную добавить в массив `MIGRATIONS`.
- `src/main.rs` — чтение env, DI, выбор бэкендов, запуск сервера.

## Инварианты

- Направление зависимостей: adapters → ports; ports ничего не знают об adapters.
- Домен и репозитории **синхронные**; HTTP-хендлеры async и переходят через `tokio::task::spawn_blocking`.
- Все репозитории инжектятся как `Arc<dyn Trait>`, собираются в `main.rs`.
- Бэкенд хранилища выбирается **по каждой сущности** (`SESSION_STORAGE` / `TICKET_STORAGE` / `CREDENTIALS_STORAGE`); PostgreSQL всё равно обязателен, если включён OIDC (auth codes только в PG).
- OIDC опционален: при отсутствии ключей/клиентов сервер стартует, а OIDC-роуты отдают 503.
- Не менять архитектуру без обсуждения; не использовать `sudo`.

Детали: стек и пины версий — `mem:tech_stack`; команды сборки/запуска/миграций — `mem:suggested_commands`; стиль кода и правила именования — `mem:conventions`; что прогонять по завершении задачи — `mem:task_completion`.
