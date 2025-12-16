// Trait для репозитория пользователей

use crate::ports::{entities::User, error::Error};

pub trait UserRepository: Send + Sync {
    fn get_by_username(&self, username: &str) -> Result<User, Error>;
    fn get(&self, user_id: &str) -> Result<User, Error>;
    fn create(&self, username: &str) -> Result<(), Error>;

    // Telegram methods
    /// Получить пользователя по Telegram ID
    fn get_by_telegram_id(&self, telegram_id: i64) -> Result<User, Error>;

    /// Создать нового пользователя через Telegram
    fn create_telegram_user(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<User, Error>;

    /// Обновить Telegram данные существующего пользователя
    fn update_telegram_data(
        &self,
        user_id: &str,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<(), Error>;
}
