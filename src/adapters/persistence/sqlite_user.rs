use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::ports::{entities::User, error::Error, user_repository::UserRepository};

pub struct SqliteUserRepository {
    pool: Pool<SqliteConnectionManager>,
}

impl SqliteUserRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        SqliteUserRepository { pool }
    }

    pub fn create_table(&self) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE,
                telegram_id INTEGER UNIQUE,
                telegram_username TEXT,
                telegram_first_name TEXT,
                telegram_last_name TEXT,
                telegram_auth_date INTEGER,
                created_at TEXT DEFAULT (datetime('now')),
                CHECK (username IS NOT NULL OR telegram_id IS NOT NULL)
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
            CREATE INDEX IF NOT EXISTS idx_users_telegram_id ON users(telegram_id);",
        )
        .map_err(|e| Error::Repository(e.to_string()))?;

        Ok(())
    }
}

fn row_to_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        telegram_id: row.get(2)?,
        telegram_username: row.get(3)?,
        telegram_first_name: row.get(4)?,
        telegram_last_name: row.get(5)?,
    })
}

const SELECT_USER: &str =
    "SELECT id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name FROM users";

impl UserRepository for SqliteUserRepository {
    fn get_by_username(&self, username: &str) -> Result<User, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.query_row(
            &format!("{SELECT_USER} WHERE username = ?1"),
            rusqlite::params![username],
            row_to_user,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::UserNotFound,
            _ => Error::Repository(e.to_string()),
        })
    }

    fn get(&self, user_id: &str) -> Result<User, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.query_row(
            &format!("{SELECT_USER} WHERE id = ?1"),
            rusqlite::params![user_id],
            row_to_user,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::UserNotFound,
            _ => Error::Repository(e.to_string()),
        })
    }

    fn create(&self, username: &str) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let user_id = uuid::Uuid::new_v4().to_string();

        let rows = conn
            .execute(
                "INSERT OR IGNORE INTO users (id, username) VALUES (?1, ?2)",
                rusqlite::params![user_id, username],
            )
            .map_err(|e| Error::Repository(e.to_string()))?;

        if rows == 0 {
            return Err(Error::UserAlreadyExists);
        }

        Ok(())
    }

    fn get_by_telegram_id(&self, telegram_id: i64) -> Result<User, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        conn.query_row(
            &format!("{SELECT_USER} WHERE telegram_id = ?1"),
            rusqlite::params![telegram_id],
            row_to_user,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::UserNotFound,
            _ => Error::Repository(e.to_string()),
        })
    }

    fn create_telegram_user(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<User, Error> {
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let user_id = uuid::Uuid::new_v4().to_string();
        let generated_username = telegram_username
            .clone()
            .map(|u| format!("tg_{u}"))
            .unwrap_or_else(|| format!("tg_{telegram_id}"));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO users (id, username, telegram_id, telegram_username, telegram_first_name, telegram_last_name, telegram_auth_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![user_id, generated_username, telegram_id, telegram_username, first_name, last_name, now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
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
        let conn = self
            .pool
            .get()
            .map_err(|e| Error::Repository(e.to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let rows = conn
            .execute(
                "UPDATE users SET telegram_id = ?1, telegram_username = ?2, telegram_first_name = ?3, telegram_last_name = ?4, telegram_auth_date = ?5 WHERE id = ?6",
                rusqlite::params![telegram_id, telegram_username, first_name, last_name, now, user_id],
            )
            .map_err(|e| Error::Repository(e.to_string()))?;

        if rows == 0 {
            return Err(Error::UserNotFound);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_pool() -> Pool<SqliteConnectionManager> {
        let manager = SqliteConnectionManager::memory()
            .with_init(|c| c.execute_batch("PRAGMA foreign_keys = ON;"));
        Pool::builder().max_size(1).build(manager).unwrap()
    }

    #[test]
    fn test_sqlite_user_repository() {
        let pool = setup_test_pool();
        let repo = SqliteUserRepository::new(pool);
        repo.create_table().unwrap();

        repo.create("alice").unwrap();

        let user = repo.get_by_username("alice").unwrap();
        assert_eq!(user.username, "alice");

        let user2 = repo.get(&user.id).unwrap();
        assert_eq!(user2.username, "alice");

        // Дубликат
        assert!(repo.create("alice").is_err());
    }

    #[test]
    fn test_sqlite_telegram_user() {
        let pool = setup_test_pool();
        let repo = SqliteUserRepository::new(pool);
        repo.create_table().unwrap();

        let user = repo
            .create_telegram_user(12345, Some("bob".to_string()), Some("Bob".to_string()), None)
            .unwrap();
        assert_eq!(user.username, "tg_bob");
        assert_eq!(user.telegram_id, Some(12345));

        let found = repo.get_by_telegram_id(12345).unwrap();
        assert_eq!(found.id, user.id);
    }
}
