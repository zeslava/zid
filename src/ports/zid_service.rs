// Trait для аутентификации

use crate::ports::{
    entities::{Ticket, VerificationResult},
    error::Error,
};

pub trait ZidService: Send + Sync {
    /// Логин по логину и паролю
    /// Если return_to не задан, тикет создаётся без привязки к сервису
    fn login(
        &self,
        username: &str,
        password: &str,
        return_to: Option<&str>,
    ) -> Result<Ticket, Error>;

    /// "Continue as ..." по существующей SSO-сессии ZID.
    ///
    /// Используется в браузерном сценарии, когда ZID уже узнал пользователя по cookie
    /// и нужно:
    /// - продлить сессию (sliding expiration),
    /// - выдать новый one-time ticket (привязанный к return_to, если он задан).
    ///
    /// Если return_to не задан, тикет создаётся без привязки к сервису.
    fn continue_as(&self, session_id: &str, return_to: Option<&str>) -> Result<Ticket, Error>;

    /// Логин через Telegram
    /// Автоматически создает пользователя, если TELEGRAM_AUTO_REGISTER=true
    /// Если return_to не задан, тикет создаётся без привязки к сервису
    fn login_telegram(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        return_to: Option<&str>,
    ) -> Result<Ticket, Error>;

    fn logout(&self, user_id: &str) -> Result<(), Error>;

    /// Верифицирует тикет и возвращает информацию о пользователе
    /// Тикет может быть использован только один раз (one-time use)
    fn verify(&self, ticket_id: &str, service_url: &str) -> Result<VerificationResult, Error>;

    /// Создать пользователя с хешированным паролем
    fn create_user(&self, username: &str, password: &str) -> Result<(), Error>;
}
