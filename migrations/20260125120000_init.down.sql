-- Down migration for initial schema

-- drop child tables first
DROP TABLE IF EXISTS tickets;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS credentials;

-- drop constraint before dropping users
ALTER TABLE users DROP CONSTRAINT IF EXISTS check_auth_method;
DROP TABLE IF EXISTS users;
