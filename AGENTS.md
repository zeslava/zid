# AGENTS.md — Rules for AI Agents

Instructions for AI agents working on the ZID project.

## Project Overview

**ZID** — a lightweight self-hosted identity provider (IdP) written in Rust.

How it works:
1. User logs in to ZID
2. ZID issues a **one-time ticket**
3. Application verifies the ticket via `/verify` → receives `user_id`, `username`
4. Application creates its own session

### Core Entities

- **User** — user (supports username/password and Telegram)
- **Session** — SSO session (7 days, sliding expiration)
- **Ticket** — one-time ticket (5 minutes TTL)
- **Credentials** — user credentials (Argon2 hashing)

## Architecture

The project uses **Hexagonal Architecture** (Ports & Adapters):

```
src/
├── ports/           # Domain interfaces (traits)
│   ├── entities.rs      # Entities: User, Session, Ticket
│   ├── error.rs         # Domain error types
│   ├── zid_service.rs   # Main service (trait)
│   └── *_repository.rs  # Repository interfaces
├── adapters/        # Infrastructure implementations
│   ├── http/            # Axum: handlers, routes, DTOs
│   ├── persistence/     # PostgreSQL, Redis, and SQLite implementations
│   └── telegram/        # Telegram Login Widget
├── application/     # Business logic
│   └── zid_app.rs       # ZidService implementation
└── main.rs          # Entry point, DI
```

### Key Files

| File | Description |
|------|-------------|
| `src/ports/entities.rs` | Domain entities |
| `src/ports/error.rs` | Error types |
| `src/application/zid_app.rs` | Authentication business logic |
| `src/adapters/http/handlers.rs` | HTTP handlers |
| `src/adapters/http/routes.rs` | API routes |
| `src/adapters/http/dto.rs` | HTTP DTOs |

## Development Rules

### Architecture Rules

1. **Dependency direction**: Adapters depend on Ports, never the reverse
   ```rust
   // adapters/persistence/postgres_user.rs
   use crate::ports::{entities::User, error::Error, user_repository::UserRepository};
   ```

2. **New repositories**:
   - Trait defined in `src/ports/`
   - Implementations in `src/adapters/persistence/` (postgres_*, redis_*)

3. **Async/Sync boundary**: HTTP handlers (async) call synchronous domain code via `spawn_blocking`:
   ```rust
   let result = tokio::task::spawn_blocking(move || {
       state.zid.login(&req.username, &req.password, req.return_to.as_deref())
   })
   .await??;
   ```

4. **Dependency Injection**: Repositories are injected into `ZidApp` via `Arc<dyn Trait>`

### Code Conventions

1. **Documentation language**: Comments and docs in **Russian**

2. **String formatting**: Use variables directly in `format!`:
   ```rust
   // Good
   format!("User {username} not found")
   // Avoid
   format!("User {} not found", username)
   ```

3. **Naming**:
   - Repositories: `PostgresXxxRepository`, `RedisXxxRepository`, `SqliteXxxRepository`
   - Traits: `XxxRepository`, `XxxService`
   - DTOs: `XxxRequest`, `XxxResponse`
   - Errors: PascalCase (`UserNotFound`, `TicketExpired`)

4. **Error handling**:
   - Domain errors via `ports::error::Error`
   - DB errors mapped to domain errors in repositories
   - HTTP errors via `HttpError` wrapper

5. **Constructors**: Use `new()` methods:
   ```rust
   impl PostgresUserRepository {
       pub fn new(pool: Pool<...>) -> Self {
           PostgresUserRepository { pool }
       }
   }
   ```

6. **UUID**: Generate via `uuid::Uuid::new_v4().to_string()`

### Testing

1. **Tests with external dependencies**: Mark with `#[ignore]`
   ```rust
   #[test]
   #[ignore] // Requires running PostgreSQL
   fn test_user_repository() { ... }
   ```

2. **Helper functions**: `setup_test_*()` for test infrastructure

3. **Placement**: `#[cfg(test)] mod tests` at the end of the implementation file

## Development Commands

```bash
# Docker
task up          # Start Docker services
task down        # Stop Docker services

# Build and run
task build       # Build Rust application
task run         # Run application locally
task cross-freebsd-aarch64  # Cross-compile for FreeBSD aarch64 (from Linux amd64), see docs/FREEBSD_SETUP.md

# Database
task migrate     # Run migrations (sqlx-cli)

# Testing
./scripts/test.sh # E2E tests
cargo test        # Unit tests
```

Additional via docker compose:

```bash
docker compose logs -f zid-app  # Application logs
docker compose ps               # Service status
```

## Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `SESSION_STORAGE` | `postgres` (default), `redis`, `sqlite` | Session storage |
| `TICKET_STORAGE` | `postgres` (default), `redis`, `sqlite` | Ticket storage |
| `CREDENTIALS_STORAGE` | `postgres` (default), `redis`, `sqlite` | Credentials storage |
| `SQLITE_PATH` | file path | SQLite database file (default: `zid.db`) |
| `TRUSTED_DOMAINS` | comma-separated | Trusted domains for return_to |
| `ZID_COOKIE_SECURE` | `auto`, `true`, `false` | Secure flag for cookie |
| `OIDC_ENABLED` | `true` (default), `false` | Enable OIDC/OAuth 2.0; starts without OIDC if config/keys are missing |
| `OIDC_ISSUER` | URL | Issuer base URL (discovery, JWT) |
| `OIDC_CLIENTS_FILE` | path | Client config file in YAML format (`.yaml` / `.yml`) |
| `OIDC_JWT_PRIVATE_KEY` | path to PEM | Private key for JWT signing |
| `OIDC_JWT_PUBLIC_KEY` | path to PEM | Public key (JWKS, verification) |

Full list in `.env.example`.

## Restrictions

1. **Do not use `sudo`** — surface commands requiring sudo to the user
2. **Do not create test examples** without explicit request
3. **Do not change the architecture** without discussion (Ports & Adapters)

## HTTP API

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/` | HTML login form |
| POST | `/` | Login form submit |
| GET | `/register` | HTML registration form |
| POST | `/register` | Registration submit |
| POST | `/login` | JSON API login |
| POST | `/login/telegram` | Telegram login |
| POST | `/verify` | Ticket verification |
| POST | `/logout` | Session deletion |
| GET | `/health` | Health check |

### OIDC/OAuth 2.0 (when OIDC_ENABLED=true)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/.well-known/openid-configuration` | Discovery |
| GET | `/oauth/authorize` | Authorization (code flow) |
| POST | `/oauth/token` | Exchange code for tokens, client_credentials |
| GET | `/oauth/userinfo` | UserInfo: Bearer **access_token** (recommended) or **id_token**; token via `Authorization: Bearer` header or `access_token` query param. Claims: sub, name, preferred_username, email (with scope profile/email). |
| GET | `/oauth/jwks` | JWKS |

Clients are configured in YAML (OIDC_CLIENTS_FILE, extension .yaml or .yml). Supports Authorization Code (+ PKCE) and Client Credentials.

## Migrations

Migrations are in `migrations/` and applied via **sqlx-cli** (`task migrate`).

### Migration Format

Migrations use `.up.sql` and `.down.sql` suffixes for rollback support:

```
migrations/
├── 001_create_users_table.up.sql
├── 001_create_users_table.down.sql
├── 002_add_telegram_support.up.sql
├── 002_add_telegram_support.down.sql
└── ...
```

### Creating New Migrations

```bash
# Create a reversible migration (with .up.sql and .down.sql)
task migrate-add NAME=description

# Or directly via sqlx-cli with -r flag
sqlx migrate add -r description
```

### Apply and Rollback

```bash
# Apply all migrations
task migrate

# Rollback last migration
task migrate-revert
```

**Important**: sqlx-cli runs `.up.sql` on apply and `.down.sql` on rollback.

### Migration Patterns

```sql
-- Use IF NOT EXISTS / IF EXISTS for idempotency
CREATE TABLE IF NOT EXISTS ...
ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...
DROP TABLE IF EXISTS ...

-- Log migration completion
DO $$
BEGIN
    RAISE NOTICE 'Migration completed successfully';
END $$;
```

### Docker and Migrations

On first PostgreSQL startup via Docker, migrations are applied automatically from `/docker-entrypoint-initdb.d`, but for production migration management use `task migrate`.

## Dependencies

Core:
- **axum** (0.8) — web framework
- **tokio** (1.48) — async runtime
- **postgres** + **r2d2** — PostgreSQL
- **redis** — Redis client
- **argon2** — password hashing
- **serde** / **serde_json** — serialization
