use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{entities::Session, error::Error, session_repository::SessionRepository};

pub struct PostgresSessionRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresSessionRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        PostgresSessionRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        conn.batch_execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id VARCHAR(36) PRIMARY KEY,
                user_id VARCHAR(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                expires_at BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);",
        )
        .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }

    /// Deletes expired sessions from the database.
    /// This method is intended to be called by a scheduled cleanup task.
    #[allow(dead_code)]
    pub fn delete_expired(&self) -> Result<u64, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let rows_deleted = conn
            .execute(
                "DELETE FROM sessions WHERE expires_at > 0 AND expires_at < $1",
                &[&current_time],
            )
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(rows_deleted as u64)
    }
}

impl SessionRepository for PostgresSessionRepository {
    fn create(&self, session_id: &str, user_id: &str, expires_at: u64) -> Result<String, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let expires_at_i64 = expires_at as i64;

        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)",
            &[&session_id, &user_id, &expires_at_i64],
        )
        .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(session_id.to_string())
    }

    fn get(&self, session_id: &str) -> Result<Session, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, user_id, expires_at FROM sessions WHERE id = $1",
                &[&session_id],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::SessionNotFound
                } else {
                    Error::RepositoryError(e.to_string())
                }
            })?;

        let expires_at: i64 = row.get(2);

        // Check if session is expired
        if expires_at > 0 {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            if current_time > expires_at {
                // Session expired, delete it and return error
                let _ = self.destroy(session_id);
                return Err(Error::SessionNotFound);
            }
        }

        Ok(Session {
            id: row.get(0),
            user_id: row.get(1),
            expires_at: expires_at as u64,
        })
    }

    fn refresh(&self, session_id: &str, new_expires_at: u64) -> Result<(), Error> {
        // Ensure the session exists and isn't expired (also handles cleanup on expiry)
        let _ = self.get(session_id)?;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let new_expires_at_i64 = new_expires_at as i64;

        let rows_affected = conn
            .execute(
                "UPDATE sessions SET expires_at = $2 WHERE id = $1",
                &[&session_id, &new_expires_at_i64],
            )
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        if rows_affected == 0 {
            return Err(Error::SessionNotFound);
        }

        Ok(())
    }

    fn destroy(&self, session_token: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        conn.execute("DELETE FROM sessions WHERE id = $1", &[&session_token])
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2_postgres::PostgresConnectionManager;

    fn setup_test_pool() -> Pool<PostgresConnectionManager<NoTls>> {
        let manager = PostgresConnectionManager::new(
            "host=localhost user=postgres password=postgres dbname=zid_test"
                .parse()
                .unwrap(),
            NoTls,
        );

        Pool::builder().max_size(5).build(manager).unwrap()
    }

    #[test]
    #[ignore] // Requires PostgreSQL running
    fn test_session_repository() {
        let pool = setup_test_pool();
        let repo = PostgresSessionRepository::new(pool);

        // Create table
        repo.create_table().unwrap();

        // Create session (assuming user exists with id "test-user-id")
        let session_id = uuid::Uuid::new_v4().to_string();
        repo.create(&session_id, "test-user-id", 0).unwrap();

        // Get session
        let session = repo.get(&session_id).unwrap();
        assert_eq!(session.id, session_id);
        assert_eq!(session.user_id, "test-user-id");

        // Note: SessionRepository doesn't expose a separate `validate` method.
        // `get()` already validates existence/expiration (and may delete expired sessions).

        // Destroy session
        repo.destroy(&session_id).unwrap();

        // Session should not exist anymore
        assert!(repo.get(&session_id).is_err());
    }

    #[test]
    #[ignore] // Requires PostgreSQL running
    fn test_session_expiration() {
        let pool = setup_test_pool();
        let repo = PostgresSessionRepository::new(pool);

        repo.create_table().unwrap();

        // Create session that expires in the past
        let session_id = uuid::Uuid::new_v4().to_string();
        let past_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 100; // 100 seconds ago

        repo.create(&session_id, "test-user-id", past_time).unwrap();

        // Getting expired session should fail
        assert!(repo.get(&session_id).is_err());
    }
}
