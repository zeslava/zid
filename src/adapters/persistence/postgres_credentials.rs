use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{credentials_repository::CredentialsRepository, error::Error};

pub struct PostgresCredentialsRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresCredentialsRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        PostgresCredentialsRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.batch_execute(
            "CREATE TABLE IF NOT EXISTS credentials (
                username VARCHAR(255) PRIMARY KEY,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

impl CredentialsRepository for PostgresCredentialsRepository {
    fn validate(&self, username: &str, password: &str) -> Result<(), Error> {
        let mut conn = self.pool.get().map_err(|e| {
            eprintln!(
                "❌ Failed to get DB connection for credentials validation: {}",
                e
            );
            Error::Repository(e.to_string())
        })?;

        let row = conn
            .query_one(
                "SELECT password_hash FROM credentials WHERE username = $1",
                &[&username],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    eprintln!("❌ Credentials not found for user '{}'", username);
                    Error::UserNotFound
                } else {
                    eprintln!(
                        "❌ DB error when fetching credentials for '{}': {}",
                        username, e
                    );
                    Error::Repository(e.to_string())
                }
            })?;

        let stored_hash: String = row.get(0);
        println!(
            "🔍 Found credentials for user '{}', hash length: {}",
            username,
            stored_hash.len(),
        );

        // Parse the stored hash
        let parsed_hash = PasswordHash::new(&stored_hash).map_err(|e| {
            eprintln!("❌ Failed to parse password hash for '{}': {}", username, e);
            Error::Repository(format!("Failed to parse password hash: {}", e))
        })?;

        println!("🔍 Password hash parsed successfully for '{}'", username);

        // Verify password against hash using Argon2
        let argon2 = Argon2::default();
        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|e| {
                eprintln!(
                    "❌ Password verification failed for '{}': {:?}",
                    username, e
                );
                Error::InvalidCredentials
            })?;

        println!("✅ Password validated successfully for '{}'", username);

        Ok(())
    }

    fn create_user(&self, username: &str, password: &str) -> Result<(), Error> {
        let mut conn = self.pool.get().map_err(|e| {
            eprintln!(
                "❌ Failed to get DB connection for creating credentials: {}",
                e
            );
            Error::Repository(e.to_string())
        })?;

        // Hash the password
        let password_hash = hash_password(password)?;

        // Insert or update credentials
        conn.execute(
            "INSERT INTO credentials (username, password_hash, updated_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT (username)
             DO UPDATE SET password_hash = $2, updated_at = CURRENT_TIMESTAMP",
            &[&username, &password_hash],
        )
        .map_err(|e| {
            eprintln!(
                "❌ Failed to insert/update credentials for '{}': {}",
                username, e
            );
            Error::Repository(e.to_string())
        })?;

        println!("✅ Credentials created/updated for user '{}'", username);
        Ok(())
    }
}

fn hash_password(password: &str) -> Result<String, Error> {
    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash password with Argon2
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Internal(format!("Failed to hash password: {}", e)))?
        .to_string();

    Ok(password_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2_postgres::PostgresConnectionManager;

    fn setup_test_pool() -> Pool<PostgresConnectionManager<NoTls>> {
        let manager = PostgresConnectionManager::new(
            "host=localhost user=postgres password=postgres dbname=zid_test"
                .parse()
                .unwrap(),
            NoTls,
        );

        Pool::builder().max_size(5).build(manager).unwrap()
    }

    #[test]
    #[ignore] // Requires PostgreSQL running
    fn test_credentials_repository() {
        let pool = setup_test_pool();
        let repo = PostgresCredentialsRepository::new(pool);

        // Create table
        repo.create_table().unwrap();

        // Create user credentials
        repo.create_user("test_user", "secret123").unwrap();

        // Validate correct password
        assert!(repo.validate("test_user", "secret123").is_ok());

        // Validate incorrect password
        assert!(repo.validate("test_user", "wrong_password").is_err());

        // Validate non-existent user
        assert!(repo.validate("non_existent", "password").is_err());
    }

    #[test]
    #[ignore] // Requires PostgreSQL running
    fn test_update_password() {
        let pool = setup_test_pool();
        let repo = PostgresCredentialsRepository::new(pool);

        repo.create_table().unwrap();

        // Create user
        repo.create_user("update_test", "old_password").unwrap();
        assert!(repo.validate("update_test", "old_password").is_ok());

        // Update password
        repo.create_user("update_test", "new_password").unwrap();

        // Old password should fail
        assert!(repo.validate("update_test", "old_password").is_err());

        // New password should work
        assert!(repo.validate("update_test", "new_password").is_ok());
    }
}
