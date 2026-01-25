-- Migration: 001_create_users_table.down.sql
-- Description: Revert users table creation
-- Created: 2024

-- Drop indexes first
DROP INDEX IF EXISTS idx_users_created_at;
DROP INDEX IF EXISTS idx_users_username;

-- Drop users table
DROP TABLE IF EXISTS users;

-- Log migration revert
DO $$
BEGIN
    RAISE NOTICE 'Migration 001_create_users_table reverted successfully';
END $$;
