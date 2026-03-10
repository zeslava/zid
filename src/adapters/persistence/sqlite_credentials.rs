use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::ports::{credentials_repository::CredentialsRepository, error::Error};

pub struct SqliteCredentialsRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl SqliteCredentialsRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        SqliteCredentialsRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS credentials (
                username TEXT PRIMARY KEY,
                password_hash TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );",
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

impl CredentialsRepository for SqliteCredentialsRepository {
    fn validate(&self, username: &str, password: &str) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let stored_hash: String = conn
            .query_row(
                "SELECT password_hash FROM credentials WHERE username = ?1",
                rusqlite::params![username],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::UserNotFound,
                _ => Error::Repository(e.to_string()),
            })?;

        let parsed_hash = PasswordHash::new(&stored_hash)
            .map_err(|e| Error::Repository(format!("Failed to parse password hash: {e}")))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| Error::InvalidCredentials)?;

        Ok(())
    }

    fn create_user(&self, username: &str, password: &str) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Internal(format!("Failed to hash password: {e}")))?
            .to_string();

        conn.execute(
            "INSERT INTO credentials (username, password_hash, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT (username)
             DO UPDATE SET password_hash = ?2, updated_at = datetime('now')",
            rusqlite::params![username, password_hash],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteCredentialsRepository {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder().max_size(1).build(manager).unwrap();
        let repo = SqliteCredentialsRepository::new(pool);
        repo.create_table().unwrap();
        repo
    }

    #[test]
    fn test_sqlite_credentials_create_and_validate() {
        let repo = setup();

        repo.create_user("alice", "secret123").unwrap();
        assert!(repo.validate("alice", "secret123").is_ok());
        assert!(repo.validate("alice", "wrong").is_err());
        assert!(repo.validate("unknown", "secret123").is_err());
    }

    #[test]
    fn test_sqlite_credentials_update_password() {
        let repo = setup();

        repo.create_user("bob", "old_pass").unwrap();
        assert!(repo.validate("bob", "old_pass").is_ok());

        repo.create_user("bob", "new_pass").unwrap();
        assert!(repo.validate("bob", "old_pass").is_err());
        assert!(repo.validate("bob", "new_pass").is_ok());
    }
}
