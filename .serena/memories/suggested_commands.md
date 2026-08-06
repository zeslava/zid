# Команды

Основной раннер — **Taskfile** (`task -l` покажет список). `.env` и `.env.local` подгружаются автоматически задачами Task, но `cargo run` их сам не читает.

```bash
task build                 # cargo build --release
task run                   # build + ./target/release/zid
task up / task down        # docker compose (postgres, redis, zid-app)
task migrate               # sqlx migrate run (нужен sqlx-cli)
task migrate-revert
task migrate-add NAME=desc # создаёт пару .up.sql/.down.sql
task oidc-gen-keys         # RSA PEM для JWT
task oidc-gen-secret       # случайный секрет клиента
task cross-freebsd-aarch64 # кросс-сборка (нужен cross + локально собранный образ, см. вывод задачи)
task install-freebsd       # запускает doas — предложить пользователю, не запускать самому
```

Тесты:
```bash
cargo test                        # только без внешних зависимостей
cargo test -- --include-ignored   # требует поднятые PostgreSQL/Redis
./scripts/test.sh                 # E2E основного флоу, сервер на localhost:5555
./scripts/test-oidc.sh            # E2E OIDC
```

Управление OIDC-клиентами — через сам бинарник: `zid oidc-client list|add -f oidc_clients.yaml` (без флагов у `add` — интерактивный режим).

Прочее: `docker compose logs -f zid-app`. Миграции применяются и автоматически на старте сервера (вшиты в бинарник), sqlx-cli нужен только для ручной работы/rollback.
