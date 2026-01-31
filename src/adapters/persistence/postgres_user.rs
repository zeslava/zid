use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::ports::{entities::User, error::Error, user_repository::UserRepository};

pub struct PostgresUserRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresUserRepository {
    pub fn new(pool: Pool<PostgresConnectionManager<NoTls>>) -> Self {
        PostgresUserRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.batch_execute(
            "CREATE TABLE IF NOT EXISTS users (
                id VARCHAR(36) PRIMARY KEY,
                username VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);",
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

impl UserRepository for PostgresUserRepository {
    fn get_by_username(&self, username: &str) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name
                 FROM users WHERE username = $1",
                &[&username],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::UserNotFound
                } else {
                    Error::Repository(e.to_string())
                }
            })?;

        Ok(User {
            id: row.get(0),
            username: row.get(1),
            telegram_id: row.get(2),
            telegram_username: row.get(3),
            telegram_first_name: row.get(4),
            telegram_last_name: row.get(5),
        })
    }

    fn get(&self, user_id: &str) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name
                 FROM users WHERE id = $1",
                &[&user_id]
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::UserNotFound
                } else {
                    Error::Repository(e.to_string())
                }
            })?;

        Ok(User {
            id: row.get(0),
            username: row.get(1),
            telegram_id: row.get(2),
            telegram_username: row.get(3),
            telegram_first_name: row.get(4),
            telegram_last_name: row.get(5),
        })
    }

    fn create(&self, username: &str) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;
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
                Error::Repository(e.to_string())
            }
        })?;

        Ok(())
    }

    fn get_by_telegram_id(&self, telegram_id: i64) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let row = conn
            .query_one(
                "SELECT id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name
                 FROM users WHERE telegram_id = $1",
                &[&telegram_id],
            )
            .map_err(|e| {
                if e.to_string()
                    .contains("query returned an unexpected number of rows")
                {
                    Error::UserNotFound
                } else {
                    Error::Repository(e.to_string())
                }
            })?;

        Ok(User {
            id: row.get(0),
            username: row.get(1),
            telegram_id: row.get(2),
            telegram_username: row.get(3),
            telegram_first_name: row.get(4),
            telegram_last_name: row.get(5),
        })
    }

    fn create_telegram_user(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<User, Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let user_id = uuid::Uuid::new_v4().to_string();

        // Генерируем username на основе Telegram данных
        // Формат: tg_{telegram_id} или tg_{telegram_username}
        let generated_username = telegram_username
            .clone()
            .map(|u| format!("tg_{}", u))
            .unwrap_or_else(|| format!("tg_{}", telegram_id));

        conn.execute(
            "INSERT INTO users (id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name, telegram_auth_date)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[
                &user_id,
                &generated_username,
                &telegram_id,
                &telegram_username,
                &first_name,
                &last_name,
                &(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64),
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("duplicate key")
                || e.to_string().contains("unique constraint")
            {
                Error::UserAlreadyExists
            } else {
                Error::Repository(e.to_string())
            }
        })?;

        Ok(User {
            id: user_id,
            username: generated_username,
            telegram_id: Some(telegram_id),
            telegram_username,
            telegram_first_name: first_name,
            telegram_last_name: last_name,
        })
    }

    fn update_telegram_data(
        &self,
        user_id: &str,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<(), Error> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let rows_affected = conn
            .execute(
                "UPDATE users
                 SET telegram_id = $1,
                     telegram_username = $2,
                     telegram_first_name = $3,
                     telegram_last_name = $4,
                     telegram_auth_date = $5
                 WHERE id = $6",
                &[
                    &telegram_id,
                    &telegram_username,
                    &first_name,
                    &last_name,
                    &(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64),
                    &user_id,
                ],
            )
            .map_err(|e| Error::Repository(e.to_string()))?;

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
        let repo = PostgresUserRepository::new(pool);

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

        // Note: delete is not supported by UserRepository in this project.
        // If you need cleanup, remove the row manually in the test DB.
    }
}
