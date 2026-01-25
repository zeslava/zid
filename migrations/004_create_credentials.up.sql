-- Migration: 004_create_credentials.sql
-- Description: Create credentials table for PostgreSQL storage
-- Created: 2024

-- Create credentials table
CREATE TABLE IF NOT EXISTS credentials (
    username VARCHAR(255) PRIMARY KEY,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Log migration
DO $$
BEGIN
    RAISE NOTICE 'Migration 004_create_credentials.sql completed successfully';
END $$;
