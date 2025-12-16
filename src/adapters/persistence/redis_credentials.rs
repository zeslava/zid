use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use redis::Commands;

use crate::ports::{credentials_repository::CredentialsRepository, error::Error};

pub struct RedisCredentialsRepository {
    client: redis::Client,
}

impl RedisCredentialsRepository {
    pub fn new(client: redis::Client) -> Self {
        RedisCredentialsRepository { client }
    }
}

impl CredentialsRepository for RedisCredentialsRepository {
    fn validate(&self, username: &str, password: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let key = format!("credentials:username:{}", username);

        // Get stored password hash
        let stored_hash: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        match stored_hash {
            Some(hash) => {
                // Parse the stored hash
                let parsed_hash = PasswordHash::new(&hash).map_err(|e| {
                    Error::RepositoryError(format!("Failed to parse password hash: {}", e))
                })?;

                // Verify password against hash using Argon2
                let argon2 = Argon2::default();
                argon2
                    .verify_password(password.as_bytes(), &parsed_hash)
                    .map_err(|_| Error::InvalidCredentials)?;

                Ok(())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn create_user(&self, username: &str, password: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let key = format!("credentials:username:{}", username);

        // Hash the password
        let password_hash = hash_password(password)?;

        // Store in Redis
        let _: () = conn
            .set(&key, &password_hash)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

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
        .map_err(|e| Error::InternalError(format!("Failed to hash password: {}", e)))?
        .to_string();

    Ok(password_hash)
}
