-- Migration: 001_create_users_table.sql
-- Description: Create users table with indexes
-- Created: 2024

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id VARCHAR(36) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index for faster username lookups
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Create index for created_at (useful for analytics)
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);

-- Log migration
DO $$
BEGIN
    RAISE NOTICE 'Migration 001_create_users_table.sql completed successfully';
END $$;
