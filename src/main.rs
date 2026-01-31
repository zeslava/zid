use std::sync::Arc;

use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::{
    adapters::http::{handlers::RouterState, routes},
    application::zid_app::ZidApp,
    ports::{
        credentials_repository::CredentialsRepository, session_repository::SessionRepository,
        ticket_repository::TicketRepository, zid_service::ZidService,
    },
};

mod adapters;
mod application;
mod ports;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Starting ZID CAS Server...");

    // Read configuration from environment variables
    let pg_host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let pg_port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let pg_db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "zid".to_string());
    let pg_user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
    let pg_password = std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "5555".to_string());

    // Storage backend configuration
    // Options: "postgres" or "redis" (default for sessions/tickets due to TTL support)
    let session_storage = std::env::var("SESSION_STORAGE").unwrap_or_else(|_| "redis".to_string());
    let ticket_storage = std::env::var("TICKET_STORAGE").unwrap_or_else(|_| "redis".to_string());
    let credentials_storage =
        std::env::var("CREDENTIALS_STORAGE").unwrap_or_else(|_| "postgres".to_string());

    // Логи конфигурации подключения (без паролей)
    println!("🔌 PostgreSQL адрес: {}:{}/{}", pg_host, pg_port, pg_db);
    println!("🔌 Redis адрес: {}", redis_url);
    println!(
        "🔧 Хранилища: sessions={}, tickets={}, credentials={}",
        session_storage, ticket_storage, credentials_storage
    );

    // Configure PostgreSQL connection pool (sync)
    let pg_connection_string = format!(
        "host={} port={} dbname={} user={} password={}",
        pg_host, pg_port, pg_db, pg_user, pg_password
    );

    let pg_manager = PostgresConnectionManager::new(
        pg_connection_string
            .parse()
            .expect("Invalid PostgreSQL connection string"),
        NoTls,
    );

    let pg_pool = Pool::builder()
        .max_size(15)
        .build(pg_manager)
        .expect("Failed to create PostgreSQL connection pool");

    println!(
        "✅ PostgreSQL connection pool created ({}:{})",
        pg_host, pg_port
    );

    // Create users table if it doesn't exist
    let user_repo_init =
        adapters::persistence::postgres_user::PostgresUserRepository::new(pg_pool.clone());
    tokio::task::spawn_blocking(move || user_repo_init.create_table())
        .await
        .expect("Failed to spawn blocking task")?;
    println!("✅ Users table initialized");

    // Initialize session repository based on configuration
    let session_repository: Arc<dyn SessionRepository> = if session_storage.to_lowercase()
        == "postgres"
    {
        // Create sessions table for PostgreSQL storage
        let session_repo_init =
            adapters::persistence::postgres_session::PostgresSessionRepository::new(
                pg_pool.clone(),
            );
        let pool_clone = pg_pool.clone();
        tokio::task::spawn_blocking(move || {
            let repo =
                adapters::persistence::postgres_session::PostgresSessionRepository::new(pool_clone);
            repo.create_table()
        })
        .await
        .expect("Failed to spawn blocking task")?;
        println!("✅ Sessions table initialized (PostgreSQL)");

        Arc::new(session_repo_init)
    } else {
        // Use Redis for sessions
        let redis_client = redis::Client::open(redis_url.as_str())?;
        println!("✅ Redis client created for sessions ({})", redis_url);

        Arc::new(
            adapters::persistence::redis_session::RedisSessionRepository::new(redis_client.clone()),
        )
    };

    // Initialize ticket repository based on configuration
    let ticket_repository: Arc<dyn TicketRepository> = if ticket_storage.to_lowercase()
        == "postgres"
    {
        // Create tickets table for PostgreSQL storage
        // Note: tickets table requires sessions table to exist first
        let pool_clone = pg_pool.clone();
        tokio::task::spawn_blocking(move || {
            let repo =
                adapters::persistence::postgres_ticket::PostgresTicketRepository::new(pool_clone);
            repo.create_table()
        })
        .await
        .expect("Failed to spawn blocking task")?;
        println!("✅ Tickets table initialized (PostgreSQL)");

        Arc::new(
            adapters::persistence::postgres_ticket::PostgresTicketRepository::new(pg_pool.clone()),
        )
    } else {
        // Use Redis for tickets
        let redis_client = redis::Client::open(redis_url.as_str())?;
        println!("✅ Redis client created for tickets ({})", redis_url);

        Arc::new(
            adapters::persistence::redis_ticket::RedisTicketRepository::new(redis_client.clone()),
        )
    };

    // Initialize credentials repository based on configuration
    println!("🔧 Credentials storage backend: {}", credentials_storage);
    let creds_repository: Arc<dyn CredentialsRepository> = if credentials_storage.to_lowercase()
        == "postgres"
    {
        // Create credentials table for PostgreSQL storage
        let pool_clone = pg_pool.clone();
        tokio::task::spawn_blocking(move || {
            let repo =
                adapters::persistence::postgres_credentials::PostgresCredentialsRepository::new(
                    pool_clone,
                );
            repo.create_table()
        })
        .await
        .expect("Failed to spawn blocking task")?;
        println!("✅ Credentials table initialized (PostgreSQL)");

        Arc::new(
            adapters::persistence::postgres_credentials::PostgresCredentialsRepository::new(
                pg_pool.clone(),
            ),
        )
    } else {
        // Use Redis for credentials
        let redis_client = redis::Client::open(redis_url.as_str())?;
        println!("✅ Redis client created for credentials ({})", redis_url);

        Arc::new(
            adapters::persistence::redis_credentials::RedisCredentialsRepository::new(redis_client),
        )
    };

    // Initialize user repository
    let user_repository =
        Arc::new(adapters::persistence::postgres_user::PostgresUserRepository::new(pg_pool));

    println!("✅ All repositories initialized");

    // Create ZID application service
    let zid_application: Arc<dyn ZidService> = Arc::new(ZidApp::new(
        user_repository,
        session_repository,
        creds_repository,
        ticket_repository,
    ));

    // Setup HTTP server (async handlers with sync services)
    let router_state = RouterState::new(zid_application);
    let router = routes::create_router(router_state);

    let bind_addr = format!("{}:{}", server_host, server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", bind_addr));

    println!();
    println!("🚀 ZID CAS Server listening on http://{}", bind_addr);
    println!(
        "   - PostgreSQL: {}:{}/{} (sync with r2d2 pool)",
        pg_host, pg_port, pg_db
    );
    println!("   - Sessions storage: {}", session_storage);
    println!("   - Tickets storage: {}", ticket_storage);
    println!("   - Credentials storage: {}", credentials_storage);
    println!("   - Handlers: async with spawn_blocking for sync operations");
    println!();

    axum::serve(listener, router)
        .await
        .expect("Failed to start server");

    Ok(())
}
