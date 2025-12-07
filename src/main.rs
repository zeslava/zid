use std::sync::Arc;

use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

use crate::{
    adapters::http::{handlers::RouterState, routes},
    application::zid_app::ZidApp,
    ports::zid_service::ZidService,
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
        adapters::persistence::user_postgres::UserPostgresRepo::new(pg_pool.clone());
    tokio::task::spawn_blocking(move || user_repo_init.create_table())
        .await
        .expect("Failed to spawn blocking task")?;
    println!("✅ Database tables initialized");

    // Configure Redis client (sync)
    let redis_client = redis::Client::open(redis_url.as_str())?;
    println!("✅ Redis client created ({})", redis_url);

    // Initialize repositories (all sync)
    let user_repository = Arc::new(adapters::persistence::user_postgres::UserPostgresRepo::new(
        pg_pool,
    ));

    let session_repository = Arc::new(adapters::persistence::session_redis::SessionRedisRepo::new(
        redis_client.clone(),
    ));

    let creds_repository = Arc::new(
        adapters::persistence::credential_redis::CredentialRedisRepo::new(redis_client.clone()),
    );

    let ticket_repository = Arc::new(adapters::persistence::ticket_redis::TicketRedisRepo::new(
        redis_client.clone(),
    ));

    println!("✅ Repositories initialized");

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
    println!("   - Redis: {} (sync)", redis_url);
    println!("   - Handlers: async with spawn_blocking for sync operations");
    println!();

    axum::serve(listener, router)
        .await
        .expect("Failed to start server");

    Ok(())
}
