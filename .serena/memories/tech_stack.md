# Стек

- Rust, **edition 2024**, пакет `zid` (Cargo, без workspace).
- HTTP: axum 0.8 (feature `macros`), tokio 1.x с **урезанным набором фич** (`rt-multi-thread, net, sync, macros, time, signal`) — не включать `full`.
- Хранилища: `postgres` 0.19 + `r2d2_postgres` (синхронный клиент, пул r2d2), `redis` 0.27, `rusqlite` 0.34 (`bundled`) + `r2d2_sqlite`.
- Пароли: `argon2` 0.5. Telegram-подпись: `hmac`/`sha2`/`hex`.
- OIDC: `jsonwebtoken` 9 (RS256), `rsa` 0.9 (`default-features = false`, только `pem`), `serde_yaml` 0.9, `base64` 0.22.
- CLI: `clap` 4 (derive) + `inquire` 0.7 (интерактивный ввод).
- Логи: `tracing` + `tracing-subscriber` с `EnvFilter` (по умолчанию `info`, переопределяется `RUST_LOG`).
- Статика: `rust-embed` 8 **без feature compression** — zstd-sys ломает сборку под `aarch64-unknown-freebsd`. Не добавлять compression.
- Ряд зависимостей намеренно с `default-features = false` (url, uuid, rsa) ради размера бинарника; комментарии в `Cargo.toml` объясняют почему — не «чинить» их.
- release-профиль: `lto = true`, `codegen-units = 1`, `strip = true`.
- Целевые платформы: Linux (dev) и **FreeBSD aarch64** (прод, кросс-сборка через `cross`, см. `docs/FREEBSD_SETUP.md`). Локальные копии rc.d-скрипта и dail-конфигов лежат в корне (`zid.dail`, `zid-pg.dail`, `scripts/zid.rc.d`).
- Dev-зависимости для HTTP-тестов: `tower` (`util`), `http-body-util`.
