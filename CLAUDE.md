# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ZID is a CAS-like SSO authentication server written in Rust with optional OIDC/OAuth 2.0 support. Users log in → ZID issues a one-time ticket → client app verifies ticket via `/verify` → gets `user_id`/`username`.

## Build & Development Commands

```bash
# Build and run
task build                    # cargo build --release
task run                      # build + run ./target/release/zid
cargo test                    # unit tests (non-ignored only)
cargo test -- --include-ignored  # includes tests requiring PostgreSQL/Redis

# Docker infrastructure
task up                       # docker compose up -d (postgres, redis, zid-app)
task down                     # docker compose down

# Database migrations (requires sqlx-cli)
task migrate                  # apply migrations
task migrate-revert           # revert last migration
task migrate-add NAME=desc    # create new migration pair (.up.sql/.down.sql)

# OIDC key generation
task oidc-gen-keys            # generate RSA PEM key pair

# E2E tests (requires running server on localhost:5555)
./scripts/test.sh             # core auth flow
./scripts/test-oidc.sh        # OIDC flow
```

Environment is configured via `.env` (see `.env.example`). Storage backends (PostgreSQL or Redis) are switchable per-entity via `SESSION_STORAGE`, `TICKET_STORAGE`, `CREDENTIALS_STORAGE` env vars.

## Architecture: Hexagonal (Ports & Adapters)

```
adapters/http/ (async Axum handlers)
    → spawn_blocking →
ports/ (sync domain traits: ZidService, OidcService, *Repository)
    ←implemented by→
application/ (ZidApp, OidcApp, OidcJwtKeys)
    →uses→
adapters/persistence/ (PostgresXxx / RedisXxx repositories)
```

**Key rule: adapters depend on ports, never the reverse.**

- **`src/ports/`** — Domain traits and entities. `ZidService` (core auth), `OidcService` (OAuth 2.0), repository traits, `Error` enum.
- **`src/application/`** — Business logic. `ZidApp` implements `ZidService`, `OidcApp` implements `OidcService`, `oidc_jwt.rs` handles RS256 JWT signing/JWKS.
- **`src/adapters/http/`** — Axum handlers, routes, DTOs, SSO cookie management. `RouterState` holds `Arc<dyn ZidService>` + optional `Arc<dyn OidcService>`.
- **`src/adapters/persistence/`** — PostgreSQL (r2d2 sync pool) and Redis implementations for each repository.
- **`src/main.rs`** — DI wiring, env var reading, server startup.

## Code Conventions

- **Comments and docs in Russian** (project convention)
- **Async/Sync boundary**: HTTP handlers are async; domain services/repositories are sync. Bridge via `tokio::task::spawn_blocking`
- **DI pattern**: All repositories are `Arc<dyn Trait>`, wired in `main.rs`
- **Naming**: `PostgresXxxRepository`/`RedisXxxRepository`, `XxxRequest`/`XxxResponse` for DTOs, PascalCase error variants
- **String formatting**: Prefer `format!("User {username}")` over `format!("User {}", username)`
- **Error handling**: Domain errors in `ports::error::Error`, mapped to HTTP status codes via `HttpError` in handlers
- **Tests with infra deps**: Mark `#[ignore]`, place in `#[cfg(test)] mod tests` at end of file
- **OIDC is optional**: Routes return 503 if OIDC is not configured (missing keys/clients/Redis)

## Constraints

- Do not use `sudo` — surface commands requiring elevated privileges to the user
- Do not change the hexagonal architecture without discussion
- New repositories: trait in `src/ports/`, implementations in `src/adapters/persistence/`
- Migrations in `migrations/` use `IF NOT EXISTS`/`IF EXISTS` for idempotency
