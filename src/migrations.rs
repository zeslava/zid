use tracing::info;

/// Встроенные SQL-миграции, отсортированные по имени файла.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_create_users_table", include_str!("../migrations/001_create_users_table.up.sql")),
    ("002_add_telegram_support", include_str!("../migrations/002_add_telegram_support.up.sql")),
    (
        "003_create_sessions_and_tickets",
        include_str!("../migrations/003_create_sessions_and_tickets.up.sql"),
    ),
    ("004_create_credentials", include_str!("../migrations/004_create_credentials.up.sql")),
    ("005_init_placeholder", include_str!("../migrations/005_init_placeholder.up.sql")),
];

/// Применяет непримёненные миграции к PostgreSQL.
pub fn run_pg(pool: &r2d2::Pool<r2d2_postgres::PostgresConnectionManager<postgres::NoTls>>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;

    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .map_err(|e| e.to_string())?;

    for (name, sql) in MIGRATIONS {
        let applied: bool = conn
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)",
                &[name],
            )
            .map_err(|e| e.to_string())?
            .get(0);

        if applied {
            continue;
        }

        info!(migration = %name, "Applying migration");
        conn.batch_execute(sql).map_err(|e| format!("Migration {name} failed: {e}"))?;
        conn.execute(
            "INSERT INTO _migrations (name) VALUES ($1)",
            &[name],
        )
        .map_err(|e| e.to_string())?;
        info!(migration = %name, "Migration applied");
    }

    Ok(())
}
