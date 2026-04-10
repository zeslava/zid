// PostgreSQL-реализация репозитория authorization codes

use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{auth_code_repository::AuthCodeRepository, entities::AuthCode, error::Error};

pub struct PostgresAuthCodeRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresAuthCodeRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        PostgresAuthCodeRepository { pool }
    }
}

impl AuthCodeRepository for PostgresAuthCodeRepository {
    fn create(&self, auth_code: &AuthCode, _ttl_secs: u64) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let expires_at = auth_code.expires_at as i64;
        let scopes = auth_code.scopes.join(" ");

        conn.execute(
            "INSERT INTO oauth_auth_codes \
             (code, client_id, user_id, redirect_uri, code_challenge, code_challenge_method, scopes, expires_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &auth_code.code,
                &auth_code.client_id,
                &auth_code.user_id,
                &auth_code.redirect_uri,
                &auth_code.code_challenge,
                &auth_code.code_challenge_method,
                &scopes,
                &expires_at,
            ],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }

    fn get(&self, code: &str) -> Result<AuthCode, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let row = conn
            .query_opt(
                "SELECT code, client_id, user_id, redirect_uri, code_challenge, code_challenge_method, scopes, expires_at \
                 FROM oauth_auth_codes WHERE code = $1",
                &[&code],
            )
            .map_err(|e| Error::Repository(e.to_string()))?
            .ok_or(Error::InvalidGrant)?;

        let expires_at: i64 = row.get(7);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if expires_at <= now {
            let _ = self.delete(code);
            return Err(Error::InvalidGrant);
        }

        let scopes_str: String = row.get(6);
        let scopes = if scopes_str.is_empty() {
            Vec::new()
        } else {
            scopes_str.split_whitespace().map(String::from).collect()
        };

        Ok(AuthCode {
            code: row.get(0),
            client_id: row.get(1),
            user_id: row.get(2),
            redirect_uri: row.get(3),
            code_challenge: row.get(4),
            code_challenge_method: row.get(5),
            scopes,
            expires_at: expires_at as u64,
        })
    }

    fn delete(&self, code: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute("DELETE FROM oauth_auth_codes WHERE code = $1", &[&code])
            .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}
