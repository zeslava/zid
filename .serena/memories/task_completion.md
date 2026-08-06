# Завершение задачи

Специальных конфигов линтера/форматтера в репозитории нет (`rustfmt.toml`/`clippy.toml` отсутствуют — файл `cross/clippy.toml` относится к вендоренному репозиторию cross, не к проекту). Используются дефолты toolchain.

Минимум перед сдачей:
```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

Если менялись репозитории/миграции — дополнительно с поднятой инфраструктурой (`task up`):
```bash
cargo test -- --include-ignored
```

Если менялся HTTP-флоу или OIDC — E2E при запущенном сервере на localhost:5555:
```bash
./scripts/test.sh
./scripts/test-oidc.sh
```

Релизная сборка (`task build`) проверяет и `lto`/`strip`-профиль; для FreeBSD-таргета — `task cross-freebsd-aarch64`.
