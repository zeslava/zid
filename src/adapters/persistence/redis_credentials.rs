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
        let mut conn = self.client.get_connection().map_err(|e| {
            eprintln!(
                "❌ Failed to get Redis connection for credentials validation: {}",
                e
            );
            Error::RepositoryError(e.to_string())
        })?;
        let key = format!("credentials:username:{}", username);

        // Get stored password hash
        let stored_hash: Option<String> = conn.get(&key).map_err(|e| {
            eprintln!(
                "❌ Redis error when fetching credentials for '{}': {}",
                username, e
            );
            Error::RepositoryError(e.to_string())
        })?;

        match stored_hash {
            Some(hash) => {
                println!(
                    "🔍 Found credentials in Redis for user '{}', hash length: {}",
                    username,
                    hash.len()
                );
                // Parse the stored hash
                let parsed_hash = PasswordHash::new(&hash).map_err(|e| {
                    eprintln!("❌ Failed to parse password hash for '{}': {}", username, e);
                    Error::RepositoryError(format!("Failed to parse password hash: {}", e))
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
            None => {
                eprintln!("❌ Credentials not found in Redis for user '{}'", username);
                Err(Error::UserNotFound)
            }
        }
    }

    fn create_user(&self, username: &str, password: &str) -> Result<(), Error> {
        let mut conn = self.client.get_connection().map_err(|e| {
            eprintln!(
                "❌ Failed to get Redis connection for creating credentials: {}",
                e
            );
            Error::RepositoryError(e.to_string())
        })?;
        let key = format!("credentials:username:{}", username);

        // Hash the password
        let password_hash = hash_password(password)?;

        // Store in Redis
        let _: () = conn.set(&key, &password_hash).map_err(|e| {
            eprintln!(
                "❌ Failed to set credentials in Redis for '{}': {}",
                username, e
            );
            Error::RepositoryError(e.to_string())
        })?;

        println!(
            "✅ Credentials created/updated in Redis for user '{}'",
            username
        );
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
