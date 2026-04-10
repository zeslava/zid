-- Revert: 006_create_oauth_auth_codes.sql

DROP INDEX IF EXISTS idx_oauth_auth_codes_expires_at;
DROP TABLE IF EXISTS oauth_auth_codes;
