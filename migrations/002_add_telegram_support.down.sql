-- Migration: 002_add_telegram_support.down.sql
-- Description: Revert Telegram authentication support
-- Created: 2024

-- Drop constraint
ALTER TABLE users DROP CONSTRAINT IF EXISTS check_auth_method;

-- Restore username NOT NULL constraint
ALTER TABLE users ALTER COLUMN username SET NOT NULL;

-- Drop indexes
DROP INDEX IF EXISTS idx_users_telegram_id;

-- Remove Telegram columns
ALTER TABLE users
DROP COLUMN IF EXISTS telegram_id,
DROP COLUMN IF EXISTS telegram_username,
DROP COLUMN IF EXISTS telegram_first_name,
DROP COLUMN IF EXISTS telegram_last_name,
DROP COLUMN IF EXISTS telegram_photo_url,
DROP COLUMN IF EXISTS telegram_auth_date;

-- Log migration revert
DO $$
BEGIN
    RAISE NOTICE 'Migration 002_add_telegram_support reverted successfully';
END $$;
