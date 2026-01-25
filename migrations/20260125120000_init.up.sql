-- Initial schema (users, telegram fields, sessions, tickets, credentials)
-- Generated for sqlx-cli

-- users
CREATE TABLE IF NOT EXISTS users (
    id VARCHAR(36) PRIMARY KEY,
    username VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);

-- telegram support
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS telegram_id BIGINT UNIQUE,
    ADD COLUMN IF NOT EXISTS telegram_username VARCHAR(255),
    ADD COLUMN IF NOT EXISTS telegram_first_name VARCHAR(255),
    ADD COLUMN IF NOT EXISTS telegram_last_name VARCHAR(255),
    ADD COLUMN IF NOT EXISTS telegram_photo_url TEXT,
    ADD COLUMN IF NOT EXISTS telegram_auth_date BIGINT;
CREATE INDEX IF NOT EXISTS idx_users_telegram_id ON users(telegram_id);
ALTER TABLE users ALTER COLUMN username DROP NOT NULL;
ALTER TABLE users ADD CONSTRAINT IF NOT EXISTS check_auth_method
    CHECK (username IS NOT NULL OR telegram_id IS NOT NULL);

-- sessions
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- tickets
CREATE TABLE IF NOT EXISTS tickets (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    service_url TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    consumed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_tickets_session_id ON tickets(session_id);
CREATE INDEX IF NOT EXISTS idx_tickets_expires_at ON tickets(expires_at);
CREATE INDEX IF NOT EXISTS idx_tickets_consumed ON tickets(consumed);

-- credentials
CREATE TABLE IF NOT EXISTS credentials (
    username VARCHAR(255) PRIMARY KEY,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
