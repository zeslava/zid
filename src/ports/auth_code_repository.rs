// Trait для репозитория authorization codes (одноразовое использование)

use crate::ports::{entities::AuthCode, error::Error};

pub trait AuthCodeRepository: Send + Sync {
    /// Создать authorization code (TTL ~5–10 мин)
    fn create(&self, auth_code: &AuthCode, ttl_secs: u64) -> Result<(), Error>;

    /// Получить код по значению (для обмена на токены)
    fn get(&self, code: &str) -> Result<AuthCode, Error>;

    /// Удалить код после использования (one-time)
    fn delete(&self, code: &str) -> Result<(), Error>;
}
