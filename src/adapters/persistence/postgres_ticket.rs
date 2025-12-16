use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{entities::Ticket, error::Error, ticket_repository::TicketRepository};

pub struct PostgresTicketRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresTicketRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        PostgresTicketRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        conn.batch_execute(
            "CREATE TABLE IF NOT EXISTS tickets (
                id VARCHAR(36) PRIMARY KEY,
                session_id VARCHAR(36) NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                service_url TEXT NOT NULL,
                expires_at BIGINT NOT NULL,
                consumed BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_tickets_session_id ON tickets(session_id);
            CREATE INDEX IF NOT EXISTS idx_tickets_expires_at ON tickets(expires_at);
            CREATE INDEX IF NOT EXISTS idx_tickets_consumed ON tickets(consumed);",
        )
        .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }

    /// Delete expired tickets (cleanup utility)
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

        let rows_affected = conn
            .execute(
                "DELETE FROM tickets WHERE expires_at > 0 AND expires_at < $1",
                &[&current_time],
            )
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(rows_affected)
    }

    /// Delete all consumed tickets (cleanup utility)
    #[allow(dead_code)]
    pub fn delete_consumed(&self) -> Result<u64, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let rows_affected = conn
            .execute("DELETE FROM tickets WHERE consumed = TRUE", &[])
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(rows_affected)
    }
}

impl TicketRepository for PostgresTicketRepository {
    fn create(
        &self,
        session_id: &str,
        service_url: &str,
        expires_at: u64,
    ) -> Result<Ticket, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let ticket_id = uuid::Uuid::new_v4().to_string();
        let expires_at_i64 = expires_at as i64;

        conn.execute(
            "INSERT INTO tickets (id, session_id, service_url, expires_at, consumed) VALUES ($1, $2, $3, $4, FALSE)",
            &[&ticket_id, &session_id, &service_url, &expires_at_i64],
        )
        .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(Ticket {
            id: ticket_id,
            session_id: session_id.to_string(),
            service_url: service_url.to_string(),
            expires_at,
            consumed: false,
        })
    }

    fn get(&self, ticket_id: &str) -> Result<Ticket, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, session_id, service_url, expires_at, consumed FROM tickets WHERE id = $1",
                &[&ticket_id],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::TicketNotFound
                } else {
                    Error::RepositoryError(e.to_string())
                }
            })?;

        let expires_at: i64 = row.get(3);

        // Check if ticket is expired
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if expires_at > 0 && current_time > expires_at {
            // Ticket expired, delete it and return error
            let _ = self.delete(ticket_id);
            return Err(Error::TicketExpired);
        }

        Ok(Ticket {
            id: row.get(0),
            session_id: row.get(1),
            service_url: row.get(2),
            expires_at: expires_at as u64,
            consumed: row.get(4),
        })
    }

    fn delete(&self, ticket_id: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let _rows_affected = conn
            .execute("DELETE FROM tickets WHERE id = $1", &[&ticket_id])
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        // Note: We don't check rows_affected because deleting a non-existent ticket is OK
        // (idempotent operation)

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
    #[ignore] // Requires PostgreSQL running with sessions table
    fn test_ticket_repository() {
        let pool = setup_test_pool();
        let repo = PostgresTicketRepository::new(pool);

        // Create table
        repo.create_table().unwrap();

        // Create ticket (assuming session exists with id "test-session-id")
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 300; // 5 minutes from now

        let ticket = repo
            .create("test-session-id", "https://example.com", expires_at)
            .unwrap();

        assert!(!ticket.consumed);
        assert_eq!(ticket.service_url, "https://example.com");

        // Get ticket
        let fetched = repo.get(&ticket.id).unwrap();
        assert_eq!(fetched.id, ticket.id);
        assert!(!fetched.consumed);

        // Note: TicketRepository doesn't expose exists/consume in the current design.
        // "One-time-use" behavior is implemented at the application/service layer by deleting tickets on verify.

        // Get ticket again (should still be present until deleted)
        let fetched2 = repo.get(&ticket.id).unwrap();
        assert_eq!(fetched2.id, ticket.id);

        // Delete ticket
        repo.delete(&ticket.id).unwrap();

        // Should not exist anymore
        assert!(repo.get(&ticket.id).is_err());
    }

    #[test]
    #[ignore] // Requires PostgreSQL running
    fn test_ticket_expiration() {
        let pool = setup_test_pool();
        let repo = PostgresTicketRepository::new(pool);

        repo.create_table().unwrap();

        // Create ticket that expires in the past
        let past_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 100; // 100 seconds ago

        let ticket = repo
            .create("test-session-id", "https://example.com", past_time)
            .unwrap();

        // Getting expired ticket should fail
        let result = repo.get(&ticket.id);
        assert!(result.is_err());
    }
}
