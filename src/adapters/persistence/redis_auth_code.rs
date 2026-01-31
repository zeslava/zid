// Redis-реализация репозитория authorization codes

use redis::Commands;

use crate::ports::{auth_code_repository::AuthCodeRepository, entities::AuthCode, error::Error};

const KEY_PREFIX: &str = "oidc:auth_code:";
const DEFAULT_TTL_SECS: u64 = 300;

pub struct RedisAuthCodeRepository {
    client: redis::Client,
}

impl RedisAuthCodeRepository {
    pub fn new(client: redis::Client) -> Self {
        RedisAuthCodeRepository { client }
    }
}

impl AuthCodeRepository for RedisAuthCodeRepository {
    fn create(&self, auth_code: &AuthCode, ttl_secs: u64) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let key = format!("{KEY_PREFIX}{}", auth_code.code);
        let serialized =
            serde_json::to_string(auth_code).map_err(|e| Error::Repository(e.to_string()))?;

        let ttl = if ttl_secs > 0 {
            ttl_secs
        } else {
            DEFAULT_TTL_SECS
        };
        let _: () = conn
            .set_ex(&key, &serialized, ttl)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }

    fn get(&self, code: &str) -> Result<AuthCode, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let key = format!("{KEY_PREFIX}{code}");
        let res: Option<String> = conn
            .get(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        match res {
            Some(data) => {
                let auth_code: AuthCode =
                    serde_json::from_str(&data).map_err(|e| Error::Repository(e.to_string()))?;
                Ok(auth_code)
            }
            None => Err(Error::InvalidGrant),
        }
    }

    fn delete(&self, code: &str) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let key = format!("{KEY_PREFIX}{code}");
        let _: () = conn
            .del(&key)
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}
