use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{entities::User, error::Error, user_repository::UserRepository};

pub struct UserPostgresRepo {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl UserPostgresRepo {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        UserPostgresRepo { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        conn.batch_execute(
            "CREATE TABLE IF NOT EXISTS users (
                id VARCHAR(36) PRIMARY KEY,
                username VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);",
        )
        .map_err(|e| Error::RepositoryError(e.to_string()))?;

        Ok(())
    }
}

impl UserRepository for UserPostgresRepo {
    fn get_by_username(&self, username: &str) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, username FROM users WHERE username = $1",
                &[&username],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::UserNotFound
                } else {
                    Error::RepositoryError(e.to_string())
                }
            })?;

        Ok(User {
            id: row.get(0),
            username: row.get(1),
        })
    }

    fn get(&self, user_id: &str) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let row = conn
            .query_one("SELECT id, username FROM users WHERE id = $1", &[&user_id])
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::UserNotFound
                } else {
                    Error::RepositoryError(e.to_string())
                }
            })?;

        Ok(User {
            id: row.get(0),
            username: row.get(1),
        })
    }

    fn create(&self, username: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;
        let user_id = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT (username) DO NOTHING",
            &[&user_id, &username],
        )
        .map_err(|e| {
            if e.to_string().contains("duplicate key")
                || e.to_string().contains("unique constraint")
            {
                Error::UserAlreadyExists
            } else {
                Error::RepositoryError(e.to_string())
            }
        })?;

        Ok(())
    }

    fn delete(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        let rows_affected = conn
            .execute("DELETE FROM users WHERE id = $1", &[&user_id])
            .map_err(|e| Error::RepositoryError(e.to_string()))?;

        if rows_affected == 0 {
            return Err(Error::UserNotFound);
        }

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
    fn test_user_repository() {
        let pool = setup_test_pool();
        let repo = UserPostgresRepo::new(pool);

        // Create table
        repo.create_table().unwrap();

        // Create user
        repo.create("test_user").unwrap();

        // Get by username
        let user = repo.get_by_username("test_user").unwrap();
        assert_eq!(user.username, "test_user");

        // Get by id
        let user2 = repo.get(&user.id).unwrap();
        assert_eq!(user2.username, "test_user");

        // Delete user
        repo.delete(&user.id).unwrap();
    }
}
