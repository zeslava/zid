// Trait для репозитория пользователей

use crate::ports::{entities::User, error::Error};

pub trait UserRepository: Send + Sync {
    fn get_by_username(&self, username: &str) -> Result<User, Error>;
    fn get(&self, user_id: &str) -> Result<User, Error>;
    fn create(&self, username: &str) -> Result<(), Error>;
    fn delete(&self, user_id: &str) -> Result<(), Error>;
}
