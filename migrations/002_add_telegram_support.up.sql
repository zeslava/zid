-- Migration: 002_add_telegram_support.sql
-- Description: Add Telegram authentication support
-- Created: 2024

-- Add Telegram ID field (nullable for existing users)
ALTER TABLE users
ADD COLUMN IF NOT EXISTS telegram_id BIGINT UNIQUE,
ADD COLUMN IF NOT EXISTS telegram_username VARCHAR(255),
ADD COLUMN IF NOT EXISTS telegram_first_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS telegram_last_name VARCHAR(255),
ADD COLUMN IF NOT EXISTS telegram_photo_url TEXT,
ADD COLUMN IF NOT EXISTS telegram_auth_date BIGINT;

-- Create index for Telegram ID lookups
CREATE INDEX IF NOT EXISTS idx_users_telegram_id ON users(telegram_id);

-- Make username nullable for Telegram-only users
-- (пользователи могут входить только через Telegram без логина/пароля)
ALTER TABLE users ALTER COLUMN username DROP NOT NULL;

-- Add constraint to ensure at least one auth method exists
-- (либо username, либо telegram_id должен быть заполнен)
ALTER TABLE users ADD CONSTRAINT check_auth_method
CHECK (username IS NOT NULL OR telegram_id IS NOT NULL);

-- Log migration
DO $$
BEGIN
    RAISE NOTICE 'Migration 002_add_telegram_support.sql completed successfully';
END $$;
