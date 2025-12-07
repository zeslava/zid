// Trait для аутентификации

use crate::ports::{
    entities::{Ticket, VerificationResult},
    error::Error,
};

pub trait ZidService: Send + Sync {
    fn login(&self, username: &str, password: &str, return_to: &str)
    -> Result<Ticket, Error>;
    fn logout(&self, user_id: &str) -> Result<(), Error>;

    /// Верифицирует тикет и возвращает информацию о пользователе
    /// Тикет может быть использован только один раз (one-time use)
    fn verify(&self, ticket_id: &str, service_url: &str)
    -> Result<VerificationResult, Error>;

    /// Создать пользователя с хешированным паролем
    fn create_user(&self, username: &str, password: &str) -> Result<(), Error>;
}
