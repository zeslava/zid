// Trait для репозитории сессий

use crate::ports::{entities::Session, error::Error};

pub trait SessionRepository: Send + Sync {
    fn create(
        &self,
        session_id: &str,
        user_id: &str,
        expires_at: u64,
    ) -> Result<String, Error>;
    fn get(&self, session_id: &str) -> Result<Session, Error>;
    fn validate(&self, session_token: &str) -> Result<(), Error>;
    fn destroy(&self, session_token: &str) -> Result<(), Error>;
}
