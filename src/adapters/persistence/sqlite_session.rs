use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::ports::{entities::Session, error::Error, session_repository::SessionRepository};

pub struct SqliteSessionRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl SqliteSessionRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        SqliteSessionRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                expires_at INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);",
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

impl SessionRepository for SqliteSessionRepository {
    fn create(&self, session_id: &str, user_id: &str, expires_at: u64) -> Result<String, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute(
            "INSERT INTO sessions (id, user_id, expires_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![session_id, user_id, expires_at as i64],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(session_id.to_string())
    }

    fn get(&self, session_id: &str) -> Result<Session, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let (id, user_id, expires_at): (String, String, i64) = conn
            .query_row(
                "SELECT id, user_id, expires_at FROM sessions WHERE id = ?1",
                rusqlite::params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::SessionNotFound,
                _ => Error::Repository(e.to_string()),
            })?;

        if expires_at > 0 && now_secs() > expires_at {
            let _ = self.destroy(session_id);
            return Err(Error::SessionNotFound);
        }

        Ok(Session {
            id,
            user_id,
            expires_at: expires_at as u64,
        })
    }

    fn refresh(&self, session_id: &str, new_expires_at: u64) -> Result<(), Error> {
        let _ = self.get(session_id)?;

        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let rows = conn
            .execute(
                "UPDATE sessions SET expires_at = ?2 WHERE id = ?1",
                rusqlite::params![session_id, new_expires_at as i64],
            )
            .map_err(|e| Error::Repository(e.to_string()))?;

        if rows == 0 {
            return Err(Error::SessionNotFound);
        }

        Ok(())
    }

    fn destroy(&self, session_token: &str) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_token],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite_user::SqliteUserRepository;

    fn setup() -> (String, SqliteSessionRepository) {
        let manager = SqliteConnectionManager::memory()
            .with_init(|c| c.execute_batch("PRAGMA foreign_keys = ON;"));
        let pool = Pool::builder().max_size(1).build(manager).unwrap();

        let user_repo = SqliteUserRepository::new(pool.clone());
        user_repo.create_table().unwrap();

        let repo = SqliteSessionRepository::new(pool.clone());
        repo.create_table().unwrap();

        use crate::ports::user_repository::UserRepository;
        user_repo.create("testuser").unwrap();
        let user = user_repo.get_by_username("testuser").unwrap();

        (user.id, repo)
    }

    #[test]
    fn test_sqlite_session_crud() {
        let (user_id, repo) = setup();

        let sid = uuid::Uuid::new_v4().to_string();
        let future = now_secs() as u64 + 3600;

        repo.create(&sid, &user_id, future).unwrap();

        let session = repo.get(&sid).unwrap();
        assert_eq!(session.user_id, user_id);

        repo.destroy(&sid).unwrap();
        assert!(repo.get(&sid).is_err());
    }

    #[test]
    fn test_sqlite_session_expiration() {
        let (user_id, repo) = setup();

        let sid = uuid::Uuid::new_v4().to_string();
        let past = now_secs() as u64 - 100;

        repo.create(&sid, &user_id, past).unwrap();
        assert!(repo.get(&sid).is_err());
    }
}
