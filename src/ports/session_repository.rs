// Trait для репозитории сессий

use crate::ports::{entities::Session, error::Error};

pub trait SessionRepository: Send + Sync {
    fn create(&self, session_id: &str, user_id: &str, expires_at: u64) -> Result<String, Error>;
    fn get(&self, session_id: &str) -> Result<Session, Error>;

    /// Sliding expiration: refresh the session's expiry time (unix seconds).
    /// Implementations should return `SessionNotFound` if the session doesn't exist (or is expired).
    fn refresh(&self, session_id: &str, new_expires_at: u64) -> Result<(), Error>;

    fn destroy(&self, session_token: &str) -> Result<(), Error>;
}
