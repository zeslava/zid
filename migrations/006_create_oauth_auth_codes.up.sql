-- Migration: 006_create_oauth_auth_codes.sql
-- Description: Create oauth_auth_codes table for OIDC authorization code storage

CREATE TABLE IF NOT EXISTS oauth_auth_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT,
    code_challenge_method TEXT,
    scopes TEXT NOT NULL DEFAULT '',
    expires_at BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_oauth_auth_codes_expires_at ON oauth_auth_codes(expires_at);

DO $$
BEGIN
    RAISE NOTICE 'Migration 006_create_oauth_auth_codes.sql completed successfully';
END $$;
