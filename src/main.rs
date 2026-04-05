use std::sync::Arc;

use clap::Parser;
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use tracing::{error, info, warn};

use crate::{
    adapters::http::{handlers::RouterState, routes},
    adapters::oidc::file_client_store::FileClientStore,
    adapters::persistence::redis_auth_code::RedisAuthCodeRepository,
    application::{oidc_app::OidcApp, oidc_jwt::OidcJwtKeys, zid_app::ZidApp},
    ports::{
        auth_code_repository::AuthCodeRepository, client_store::ClientStore,
        credentials_repository::CredentialsRepository, oidc_service::OidcService,
        session_repository::SessionRepository, ticket_repository::TicketRepository,
        user_repository::UserRepository, zid_service::ZidService,
    },
};

mod adapters;
mod application;
mod cli;
mod migrations;
mod ports;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    match args.command {
        None | Some(cli::Command::Serve) => run_server().await,
        Some(cli::Command::OidcClient { file, action }) => {
            cli::handle_oidc_client(file, action);
            Ok(())
        }
    }
}

async fn run_server() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting ZID Server...");

    // Read configuration from environment variables
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "5555".to_string());

    // Storage backend configuration
    // Options: "postgres" (default), "redis", or "sqlite"
    let session_storage =
        std::env::var("SESSION_STORAGE").unwrap_or_else(|_| "postgres".to_string());
    let ticket_storage = std::env::var("TICKET_STORAGE").unwrap_or_else(|_| "postgres".to_string());
    let credentials_storage =
        std::env::var("CREDENTIALS_STORAGE").unwrap_or_else(|_| "postgres".to_string());

    info!(
        sessions = %session_storage,
        tickets = %ticket_storage,
        credentials = %credentials_storage,
        "Storage backends configured"
    );

    // Определяем, какие бэкенды нужны
    let storages = [&session_storage, &ticket_storage, &credentials_storage];
    let need_postgres = storages.iter().any(|s| s.to_lowercase() == "postgres");
    let need_sqlite = storages.iter().any(|s| s.to_lowercase() == "sqlite");

    // PostgreSQL pool (создаём только при необходимости)
    let pg_pool = if need_postgres {
        // Приоритет: DATABASE_URL (URL-формат, совместим с sqlx-cli),
        // иначе собираем из POSTGRES_* (обратная совместимость).
        let pg_connection_string = match std::env::var("DATABASE_URL") {
            Ok(url) if !url.is_empty() => {
                info!("Connecting to PostgreSQL (DATABASE_URL)");
                url
            }
            _ => {
                let pg_host =
                    std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
                let pg_port =
                    std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
                let pg_db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "zid".to_string());
                let pg_user =
                    std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
                let pg_password =
                    std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());

                info!(host = %pg_host, port = %pg_port, db = %pg_db, "Connecting to PostgreSQL");

                format!(
                    "host={pg_host} port={pg_port} dbname={pg_db} user={pg_user} password={pg_password}"
                )
            }
        };

        let pg_manager = PostgresConnectionManager::new(
            pg_connection_string
                .parse()
                .expect("Invalid PostgreSQL connection string"),
            NoTls,
        );

        let pool = Pool::builder()
            .max_size(15)
            .build(pg_manager)
            .expect("Failed to create PostgreSQL connection pool");

        // Применяем миграции
        let migrate_pool = pool.clone();
        tokio::task::spawn_blocking(move || migrations::run_pg(&migrate_pool))
            .await
            .expect("Failed to spawn blocking task")
            .map_err(|e| anyhow::anyhow!(e))?;

        info!("PostgreSQL connection pool created");
        Some(pool)
    } else {
        None
    };

    // SQLite pool (создаём только при необходимости)
    let sqlite_pool = if need_sqlite {
        let sqlite_path = std::env::var("SQLITE_PATH").unwrap_or_else(|_| "zid.db".to_string());

        info!(path = %sqlite_path, "Connecting to SQLite");

        let manager = r2d2_sqlite::SqliteConnectionManager::file(&sqlite_path)
            .with_init(|c| c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;"));

        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .expect("Failed to create SQLite connection pool");

        // Инициализируем таблицу users в SQLite
        let user_repo_init =
            adapters::persistence::sqlite_user::SqliteUserRepository::new(pool.clone());
        tokio::task::spawn_blocking(move || user_repo_init.create_table())
            .await
            .expect("Failed to spawn blocking task")?;

        info!("SQLite connection pool created");
        Some(pool)
    } else {
        None
    };

    // Макрос-хелпер недоступен, используем замыкание для получения пулов
    let get_pg = || {
        pg_pool
            .clone()
            .expect("PostgreSQL pool required but not configured. Set POSTGRES_* env vars.")
    };
    let get_sqlite = || {
        sqlite_pool
            .clone()
            .expect("SQLite pool required but not configured. Set SQLITE_PATH env var.")
    };

    // Initialize session repository
    let session_repository: Arc<dyn SessionRepository> = match session_storage
        .to_lowercase()
        .as_str()
    {
        "sqlite" => {
            let pool = get_sqlite();
            let repo =
                adapters::persistence::sqlite_session::SqliteSessionRepository::new(pool.clone());
            tokio::task::spawn_blocking({
                let pool = pool.clone();
                move || {
                    adapters::persistence::sqlite_session::SqliteSessionRepository::new(pool)
                        .create_table()
                }
            })
            .await
            .expect("Failed to spawn blocking task")?;
            info!("Sessions initialized (SQLite)");
            Arc::new(repo)
        }
        "redis" => {
            let redis_client = redis::Client::open(redis_url.as_str())?;
            info!("Sessions initialized (Redis)");
            Arc::new(
                adapters::persistence::redis_session::RedisSessionRepository::new(redis_client),
            )
        }
        _ => {
            let pool = get_pg();
            info!("Sessions initialized (PostgreSQL)");
            Arc::new(adapters::persistence::postgres_session::PostgresSessionRepository::new(pool))
        }
    };

    // Initialize ticket repository
    let ticket_repository: Arc<dyn TicketRepository> = match ticket_storage.to_lowercase().as_str()
    {
        "sqlite" => {
            let pool = get_sqlite();
            let repo =
                adapters::persistence::sqlite_ticket::SqliteTicketRepository::new(pool.clone());
            tokio::task::spawn_blocking({
                let pool = pool.clone();
                move || {
                    adapters::persistence::sqlite_ticket::SqliteTicketRepository::new(pool)
                        .create_table()
                }
            })
            .await
            .expect("Failed to spawn blocking task")?;
            info!("Tickets initialized (SQLite)");
            Arc::new(repo)
        }
        "redis" => {
            let redis_client = redis::Client::open(redis_url.as_str())?;
            info!("Tickets initialized (Redis)");
            Arc::new(adapters::persistence::redis_ticket::RedisTicketRepository::new(redis_client))
        }
        _ => {
            let pool = get_pg();
            info!("Tickets initialized (PostgreSQL)");
            Arc::new(adapters::persistence::postgres_ticket::PostgresTicketRepository::new(pool))
        }
    };

    // Initialize credentials repository
    let creds_repository: Arc<dyn CredentialsRepository> =
        match credentials_storage.to_lowercase().as_str() {
            "sqlite" => {
                let pool = get_sqlite();
                let repo =
                    adapters::persistence::sqlite_credentials::SqliteCredentialsRepository::new(
                        pool.clone(),
                    );
                tokio::task::spawn_blocking({
                    let pool = pool.clone();
                    move || {
                        adapters::persistence::sqlite_credentials::SqliteCredentialsRepository::new(
                            pool,
                        )
                        .create_table()
                    }
                })
                .await
                .expect("Failed to spawn blocking task")?;
                info!("Credentials initialized (SQLite)");
                Arc::new(repo)
            }
            "redis" => {
                let redis_client = redis::Client::open(redis_url.as_str())?;
                info!("Credentials initialized (Redis)");
                Arc::new(
                    adapters::persistence::redis_credentials::RedisCredentialsRepository::new(
                        redis_client,
                    ),
                )
            }
            _ => {
                let pool = get_pg();
                info!("Credentials initialized (PostgreSQL)");
                Arc::new(
                    adapters::persistence::postgres_credentials::PostgresCredentialsRepository::new(
                        pool,
                    ),
                )
            }
        };

    // Initialize user repository (users всегда хранятся в том же бэкенде, что и sessions)
    let user_repository: Arc<dyn UserRepository> = if need_sqlite && !need_postgres {
        // Полностью SQLite режим
        Arc::new(adapters::persistence::sqlite_user::SqliteUserRepository::new(get_sqlite()))
    } else {
        // PostgreSQL (по умолчанию, или смешанный режим)
        Arc::new(adapters::persistence::postgres_user::PostgresUserRepository::new(get_pg()))
    };

    info!("All repositories initialized");

    // Create ZID application service
    let zid_application: Arc<dyn ZidService> = Arc::new(ZidApp::new(
        user_repository.clone(),
        session_repository,
        creds_repository,
        ticket_repository,
    ));

    // OIDC/OAuth 2.0: по умолчанию включён; при отсутствии конфига/ключей — запуск без OIDC
    let oidc_wanted = std::env::var("OIDC_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase();
    let oidc_wanted = oidc_wanted == "true" || oidc_wanted == "1";

    let mut router_state = RouterState::new(zid_application);

    if oidc_wanted {
        // Issuer — URL сервера авторизации (ZID), по которому клиенты обращаются к discovery и проверяют JWT.
        // 0.0.0.0 не подходит: клиенты не могут к нему обращаться; по умолчанию подставляем localhost.
        let issuer_host = if server_host == "0.0.0.0" || server_host == "::" {
            "localhost"
        } else {
            &server_host
        };
        let oidc_issuer = std::env::var("OIDC_ISSUER")
            .unwrap_or_else(|_| format!("http://{}:{}", issuer_host, server_port));
        let clients_file =
            std::env::var("OIDC_CLIENTS_FILE").unwrap_or_else(|_| "oidc_clients.yaml".to_string());
        let private_key_path = std::env::var("OIDC_JWT_PRIVATE_KEY")
            .unwrap_or_else(|_| "oidc_jwt_private.pem".to_string());
        let public_key_path = std::env::var("OIDC_JWT_PUBLIC_KEY")
            .unwrap_or_else(|_| "oidc_jwt_public.pem".to_string());

        let clients_path = std::path::Path::new(&clients_file);
        let client_store = match FileClientStore::from_path(clients_path) {
            Ok(store) => {
                info!(file = %clients_file, "OIDC clients loaded");
                Some(Arc::new(store) as Arc<dyn ClientStore>)
            }
            Err(e) => {
                warn!(file = %clients_file, error = %e, "OIDC disabled: failed to load clients file");
                None
            }
        };

        let auth_code_repository =
            client_store
                .as_ref()
                .and_then(|_| match redis::Client::open(redis_url.as_str()) {
                    Ok(c) => {
                        Some(Arc::new(RedisAuthCodeRepository::new(c))
                            as Arc<dyn AuthCodeRepository>)
                    }
                    Err(e) => {
                        warn!(error = %e, "OIDC disabled: Redis required for auth codes");
                        None
                    }
                });

        let jwt_keys = client_store.as_ref().and_then(|_| {
            let priv_path = std::path::Path::new(&private_key_path);
            let pub_path = std::path::Path::new(&public_key_path);
            match OidcJwtKeys::from_pem_paths(priv_path, pub_path, "zid-rs256-1") {
                Ok(k) => Some(Arc::new(k)),
                Err(e) => {
                    warn!(error = %e, "OIDC disabled: failed to load JWT keys");
                    None
                }
            }
        });

        if let (Some(client_store), Some(auth_code_repository), Some(jwt_keys)) =
            (client_store, auth_code_repository, jwt_keys)
        {
            let oidc_app: Arc<dyn OidcService> = Arc::new(OidcApp::new(
                client_store,
                auth_code_repository,
                jwt_keys,
                user_repository,
                oidc_issuer.trim_end_matches('/').to_string(),
            ));
            router_state =
                router_state.with_oidc(oidc_app, oidc_issuer.trim_end_matches('/').to_string());
            info!(issuer = %oidc_issuer, "OIDC/OAuth 2.0 enabled");
        } else {
            warn!(
                clients_file = %clients_file,
                private_key = %private_key_path,
                public_key = %public_key_path,
                "OIDC not enabled: missing config. /oauth/* will return 503"
            );
        }
    }

    // Setup HTTP server (async handlers with sync services)
    let router = routes::create_router(router_state);

    let bind_addr = format!("{}:{}", server_host, server_port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(addr = %bind_addr, error = %e, "Failed to bind");
            std::process::exit(1);
        }
    };

    info!(
        addr = %bind_addr,
        sessions = %session_storage,
        tickets = %ticket_storage,
        credentials = %credentials_storage,
        "ZID Server listening"
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("ZID Server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
        _ = terminate => info!("Received SIGTERM, shutting down..."),
    }
}
