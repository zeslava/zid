-- Migration: 004_create_credentials.down.sql
-- Description: Revert credentials table creation
-- Created: 2024

-- Drop credentials table
DROP TABLE IF EXISTS credentials;

-- Log migration revert
DO $$
BEGIN
    RAISE NOTICE 'Migration 004_create_credentials reverted successfully';
END $$;
