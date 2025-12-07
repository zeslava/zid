use redis::Commands;
use serde::{Deserialize, Serialize};

use crate::ports::{entities::Session, error::Error, session_repository::SessionRepository};

pub struct SessionRedisRepo {
    client: redis::Client,
}

impl SessionRedisRepo {
    pub fn new(client: redis::Client) -> Self {
        SessionRedisRepo { client }
    }
}

impl SessionRepository for SessionRedisRepo {
    fn create(
        &self,
        session_id: &str,
        user_id: &str,
        expires_at: u64,
    ) -> Result<String, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let key = format!("session:id:{}", session_id);

        let session = SessionDTO {
            id: session_id.to_string(),
            user_id: user_id.to_string(),
            expires_at,
        };

        let serialized = serde_json::to_string(&session)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Store session with TTL if expires_at is set
        if expires_at > 0 {
            let ttl = expires_at
                .saturating_sub(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                )
                .max(1);

            let _: () = conn
                .set_ex(&key, &serialized, ttl)
                .map_err(|e| Error::RepositoryError(e.to_string()))?;
        } else {
            let _: () = conn
                .set(&key, &serialized)
                .map_err(|e| Error::RepositoryError(e.to_string()))?;
        }

        Ok(session_id.to_string())
    }

    fn get(&self, session_id: &str) -> Result<Session, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let key = format!("session:id:{}", session_id);
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        match res {
            Some(data) => {
                let session_dto: SessionDTO = serde_json::from_str(&data)
                    .map_err(|e| Error::RepositoryError(e.to_string()))?;
                Ok(session_dto.into())
            }
            None => Err(Error::SessionNotFound),
        }
    }

    fn validate(&self, session_token: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let key = format!("session:id:{}", session_token);
        let exists: bool = conn
            .exists(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        if exists {
            Ok(())
        } else {
            Err(Error::SessionNotFound)
        }
    }

    fn destroy(&self, session_token: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let key = format!("session:id:{}", session_token);
        let _: () = conn
            .del(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct SessionDTO {
    id: String,
    user_id: String,
    expires_at: u64,
}

impl From<SessionDTO> for Session {
    fn from(dto: SessionDTO) -> Self {
        Session {
            id: dto.id,
            user_id: dto.user_id,
            expires_at: dto.expires_at,
        }
    }
}

impl From<Session> for SessionDTO {
    fn from(session: Session) -> Self {
        SessionDTO {
            id: session.id,
            user_id: session.user_id,
            expires_at: session.expires_at,
        }
    }
}
