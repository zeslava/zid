-- Migration: 003_create_sessions_and_tickets.sql
-- Description: Create sessions and tickets tables for PostgreSQL storage
-- Created: 2024

-- Create sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index for user_id lookups (find all sessions for a user)
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);

-- Create index for expiration cleanup
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Create tickets table
CREATE TABLE IF NOT EXISTS tickets (
    id VARCHAR(36) PRIMARY KEY,
    session_id VARCHAR(36) NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    service_url TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    consumed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index for session_id lookups
CREATE INDEX IF NOT EXISTS idx_tickets_session_id ON tickets(session_id);

-- Create index for expiration cleanup
CREATE INDEX IF NOT EXISTS idx_tickets_expires_at ON tickets(expires_at);

-- Create index for consumed status (useful for cleanup queries)
CREATE INDEX IF NOT EXISTS idx_tickets_consumed ON tickets(consumed);

-- Log migration
DO $$
BEGIN
    RAISE NOTICE 'Migration 003_create_sessions_and_tickets.sql completed successfully';
END $$;
