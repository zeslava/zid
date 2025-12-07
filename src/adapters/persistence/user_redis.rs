use redis::Commands;
use serde::{Deserialize, Serialize};

use crate::ports::{entities::User, error::Error, user_repository::UserRepository};

pub struct UserRedisRepo {
    client: redis::Client,
}

impl UserRedisRepo {
    pub fn new(client: redis::Client) -> Self {
        UserRedisRepo { client }
    }
}

impl UserRepository for UserRedisRepo {
    fn get_by_username(&self, username: &str) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let key = format!("user:username:{}", username);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;
                Ok(user_dto.into())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn get(&self, user_id: &str) -> Result<User, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let key = format!("user:id:{}", user_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;
                Ok(user_dto.into())
            }
            None => Err(Error::UserNotFound),
        }
    }

    fn create(&self, username: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Generate new user ID
        let user_id = uuid::Uuid::new_v4().to_string();

        // Create user
        let user = User {
            id: user_id.clone(),
            username: username.to_string(),
        };

        let user_dto: UserDTO = user.into();
        let serialized = serde_json::to_string(&user_dto)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Store by both ID and username for quick lookups
        let key_by_id = format!("user:id:{}", user_id);
        let key_by_username = format!("user:username:{}", username);

        let _: () = conn
            .set(&key_by_id, &serialized)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let _: () = conn
            .set(&key_by_username, &serialized)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }

    fn delete(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // First, get the user to find their username
        let key = format!("user:id:{}", user_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        match res {
            Some(data) => {
                let user_dto: UserDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;

                // Delete both keys
                let key_by_id = format!("user:id:{}", user_id);
                let key_by_username = format!("user:username:{}", user_dto.username);

                let _: () = conn
                    .del(&key_by_id)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;
                let _: () = conn
                    .del(&key_by_username)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;

                Ok(())
            }
            None => Err(Error::UserNotFound),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserDTO {
    id: String,
    username: String,
}

impl From<User> for UserDTO {
    fn from(user: User) -> Self {
        UserDTO {
            id: user.id,
            username: user.username,
        }
    }
}

impl From<UserDTO> for User {
    fn from(dto: UserDTO) -> Self {
        User {
            id: dto.id,
            username: dto.username,
        }
    }
}
