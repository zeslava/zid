-- Migration: 003_create_sessions_and_tickets.down.sql
-- Description: Revert sessions and tickets tables creation
-- Created: 2024

-- Drop tickets table first (due to foreign key)
DROP TABLE IF EXISTS tickets;

-- Drop sessions table
DROP TABLE IF EXISTS sessions;

-- Log migration revert
DO $$
BEGIN
    RAISE NOTICE 'Migration 003_create_sessions_and_tickets reverted successfully';
END $$;
