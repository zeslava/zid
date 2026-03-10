use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::ports::{entities::Ticket, error::Error, ticket_repository::TicketRepository};

pub struct SqliteTicketRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl SqliteTicketRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        SqliteTicketRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tickets (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                service_url TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                consumed INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_tickets_session_id ON tickets(session_id);
            CREATE INDEX IF NOT EXISTS idx_tickets_expires_at ON tickets(expires_at);
            CREATE INDEX IF NOT EXISTS idx_tickets_consumed ON tickets(consumed);",
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

impl TicketRepository for SqliteTicketRepository {
    fn create(
        &self,
        session_id: &str,
        service_url: &str,
        expires_at: u64,
    ) -> Result<Ticket, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let ticket_id = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO tickets (id, session_id, service_url, expires_at, consumed) VALUES (?1, ?2, ?3, ?4, 0)",
            rusqlite::params![ticket_id, session_id, service_url, expires_at as i64],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(Ticket {
            id: ticket_id,
            session_id: session_id.to_string(),
            service_url: service_url.to_string(),
            expires_at,
            consumed: false,
        })
    }

    fn get(&self, ticket_id: &str) -> Result<Ticket, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let (id, session_id, service_url, expires_at, consumed): (String, String, String, i64, bool) =
            conn.query_row(
                "SELECT id, session_id, service_url, expires_at, consumed FROM tickets WHERE id = ?1",
                rusqlite::params![ticket_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::TicketNotFound,
                _ => Error::Repository(e.to_string()),
            })?;

        if expires_at > 0 && now_secs() > expires_at {
            let _ = self.delete(ticket_id);
            return Err(Error::TicketExpired);
        }

        Ok(Ticket {
            id,
            session_id,
            service_url,
            expires_at: expires_at as u64,
            consumed,
        })
    }

    fn delete(&self, ticket_id: &str) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute(
            "DELETE FROM tickets WHERE id = ?1",
            rusqlite::params![ticket_id],
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::persistence::sqlite_session::SqliteSessionRepository;
    use crate::adapters::persistence::sqlite_user::SqliteUserRepository;
    use crate::ports::{
        session_repository::SessionRepository, user_repository::UserRepository,
    };

    fn setup() -> SqliteTicketRepository {
        let manager = SqliteConnectionManager::memory()
            .with_init(|c| c.execute_batch("PRAGMA foreign_keys = ON;"));
        let pool = Pool::builder().max_size(1).build(manager).unwrap();

        let user_repo = SqliteUserRepository::new(pool.clone());
        user_repo.create_table().unwrap();
        user_repo.create("testuser").unwrap();
        let user = user_repo.get_by_username("testuser").unwrap();

        let session_repo = SqliteSessionRepository::new(pool.clone());
        session_repo.create_table().unwrap();

        let future = now_secs() as u64 + 3600;
        session_repo
            .create("test-session", &user.id, future)
            .unwrap();

        let repo = SqliteTicketRepository::new(pool.clone());
        repo.create_table().unwrap();

        repo
    }

    #[test]
    fn test_sqlite_ticket_crud() {
        let repo = setup();

        let future = now_secs() as u64 + 300;
        let ticket = repo
            .create("test-session", "https://example.com", future)
            .unwrap();

        assert!(!ticket.consumed);

        let fetched = repo.get(&ticket.id).unwrap();
        assert_eq!(fetched.id, ticket.id);

        repo.delete(&ticket.id).unwrap();
        assert!(repo.get(&ticket.id).is_err());
    }

    #[test]
    fn test_sqlite_ticket_expiration() {
        let repo = setup();

        let past = now_secs() as u64 - 100;
        let ticket = repo
            .create("test-session", "https://example.com", past)
            .unwrap();

        assert!(repo.get(&ticket.id).is_err());
    }
}
