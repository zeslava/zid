// Trait для работы с учетными данными

use crate::ports::error::Error;

pub trait CredentialsRepository: Send + Sync {
    /// Validate username and password
    fn validate(&self, username: &str, password: &str) -> Result<(), Error>;

    /// Create or update user credentials with hashed password
    fn create_user(&self, username: &str, password: &str) -> Result<(), Error>;
}
