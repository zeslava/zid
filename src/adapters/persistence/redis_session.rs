use redis::Commands;
use serde::{Deserialize, Serialize};

use crate::ports::{entities::Session, error::Error, session_repository::SessionRepository};

pub struct RedisSessionRepository {
    client: redis::Client,
}

impl RedisSessionRepository {
    pub fn new(client: redis::Client) -> Self {
        RedisSessionRepository { client }
    }
}

impl SessionRepository for RedisSessionRepository {
    fn create(&self, session_id: &str, user_id: &str, expires_at: u64) -> Result<String, Error> {
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

        let serialized =
            serde_json::to_string(&session).map_err(|e| Error::RepositoryError(e.to_string()))?;

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

    fn refresh(&self, session_id: &str, new_expires_at: u64) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let key = format!("session:id:{}", session_id);

        // Load existing session (to keep user_id intact)
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let data = match res {
            Some(data) => data,
            None => return Err(Error::SessionNotFound),
        };

        let mut session_dto: SessionDTO =
            serde_json::from_str(&data).map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Update expiry in payload
        session_dto.expires_at = new_expires_at;

        let serialized = serde_json::to_string(&session_dto)
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Apply new TTL only when expiry is set
        if new_expires_at > 0 {
            let ttl = new_expires_at
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

        Ok(())
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
