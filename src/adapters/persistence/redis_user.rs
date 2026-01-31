use redis::Commands;
use serde::{Deserialize, Serialize};

use crate::ports::{entities::User, error::Error, user_repository::UserRepository};

#[allow(dead_code)]
pub struct RedisUserRepository {
    client: redis::Client,
}

impl RedisUserRepository {
    #[allow(dead_code)]
    pub fn new(client: redis::Client) -> Self {
        RedisUserRepository { client }
    }
}

impl UserRepository for RedisUserRepository {
    fn get_by_username(&self, username: &str) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;
        let key = format!("user:username:{}", username);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::Repository(e.to_string()))?;
                Ok(user_dto.into())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn get(&self, user_id: &str) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;
        let key = format!("user:id:{}", user_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::Repository(e.to_string()))?;
                Ok(user_dto.into())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn create(&self, username: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        // Generate new user ID
        let user_id = uuid::Uuid::new_v4().to_string();

        // Create user
        let user = User {
            id: user_id.clone(),
            username: username.to_string(),
            telegram_id: None,
            telegram_username: None,
            telegram_first_name: None,
            telegram_last_name: None,
        };

        let user_dto: UserDTO = user.into();
        let serialized =
            serde_json::to_string(&user_dto).map_err(|e| Error::Repository(e.to_string()))?;

        // Store by both ID and username for quick lookups
        let key_by_id = format!("user:id:{}", user_id);
        let key_by_username = format!("user:username:{}", username);

        let _: () = conn
            .set(&key_by_id, &serialized)
            .map_err(|e| Error::Repository(e.to_string()))?;
        let _: () = conn
            .set(&key_by_username, &serialized)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }

    fn get_by_telegram_id(&self, telegram_id: i64) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;
        let key = format!("user:telegram_id:{}", telegram_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::Repository(e.to_string()))?;
                Ok(user_dto.into())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn create_telegram_user(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let user_id = uuid::Uuid::new_v4().to_string();

        let generated_username = telegram_username
            .clone()
            .map(|u| format!("tg_{}", u))
            .unwrap_or_else(|| format!("tg_{}", telegram_id));

        let user = User {
            id: user_id.clone(),
            username: generated_username.clone(),
            telegram_id: Some(telegram_id),
            telegram_username: telegram_username.clone(),
            telegram_first_name: first_name.clone(),
            telegram_last_name: last_name.clone(),
        };

        let user_dto: UserDTO = user.clone().into();
        let serialized =
            serde_json::to_string(&user_dto).map_err(|e| Error::Repository(e.to_string()))?;

        // Store by ID, username, and telegram_id for quick lookups
        let key_by_id = format!("user:id:{}", user_id);
        let key_by_username = format!("user:username:{}", generated_username);
        let key_by_telegram = format!("user:telegram_id:{}", telegram_id);

        let _: () = conn
            .set(&key_by_id, &serialized)
            .map_err(|e| Error::Repository(e.to_string()))?;
        let _: () = conn
            .set(&key_by_username, &serialized)
            .map_err(|e| Error::Repository(e.to_string()))?;
        let _: () = conn
            .set(&key_by_telegram, &serialized)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(user)
    }

    fn update_telegram_data(
        &self,
        user_id: &str,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        // Get existing user
        let key = format!("user:id:{}", user_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let mut user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::Repository(e.to_string()))?;

                // Update Telegram data
                user_dto.telegram_id = Some(telegram_id);
                user_dto.telegram_username = telegram_username;
                user_dto.telegram_first_name = first_name;
                user_dto.telegram_last_name = last_name;

                let serialized = serde_json::to_string(&user_dto)
                    .map_err(|e| Error::Repository(e.to_string()))?;

                // Update all keys
                let key_by_id = format!("user:id:{}", user_id);
                let key_by_username = format!("user:username:{}", user_dto.username);
                let key_by_telegram = format!("user:telegram_id:{}", telegram_id);

                let _: () = conn
                    .set(&key_by_id, &serialized)
                    .map_err(|e| Error::Repository(e.to_string()))?;
                let _: () = conn
                    .set(&key_by_username, &serialized)
                    .map_err(|e| Error::Repository(e.to_string()))?;
                let _: () = conn
                    .set(&key_by_telegram, &serialized)
                    .map_err(|e| Error::Repository(e.to_string()))?;

                Ok(())
            }
            None => Err(Error::UserNotFound),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UserDTO {
    id: String,
    username: String,
    telegram_id: Option<i64>,
    telegram_username: Option<String>,
    telegram_first_name: Option<String>,
    telegram_last_name: Option<String>,
}

impl From<User> for UserDTO {
    fn from(user: User) -> Self {
        UserDTO {
            id: user.id,
            username: user.username,
            telegram_id: user.telegram_id,
            telegram_username: user.telegram_username,
            telegram_first_name: user.telegram_first_name,
            telegram_last_name: user.telegram_last_name,
        }
    }
}

impl From<UserDTO> for User {
    fn from(dto: UserDTO) -> Self {
        User {
            id: dto.id,
            username: dto.username,
            telegram_id: dto.telegram_id,
            telegram_username: dto.telegram_username,
            telegram_first_name: dto.telegram_first_name,
            telegram_last_name: dto.telegram_last_name,
        }
    }
}
