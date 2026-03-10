use std::sync::Arc;

use clap::Parser;
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;

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
    println!("🚀 Starting ZID Server...");

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

    println!(
        "🔧 Хранилища: sessions={}, tickets={}, credentials={}",
        session_storage, ticket_storage, credentials_storage
    );

    // Определяем, какие бэкенды нужны
    let storages = [&session_storage, &ticket_storage, &credentials_storage];
    let need_postgres = storages.iter().any(|s| s.to_lowercase() == "postgres");
    let need_sqlite = storages.iter().any(|s| s.to_lowercase() == "sqlite");

    // PostgreSQL pool (создаём только при необходимости)
    let pg_pool = if need_postgres {
        let pg_host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let pg_port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let pg_db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "zid".to_string());
        let pg_user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
        let pg_password =
            std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());

        println!("🔌 PostgreSQL: {}:{}/{}", pg_host, pg_port, pg_db);

        let pg_connection_string = format!(
            "host={pg_host} port={pg_port} dbname={pg_db} user={pg_user} password={pg_password}"
        );

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

        // Инициализируем таблицу users в PostgreSQL
        let user_repo_init =
            adapters::persistence::postgres_user::PostgresUserRepository::new(pool.clone());
        tokio::task::spawn_blocking(move || user_repo_init.create_table())
            .await
            .expect("Failed to spawn blocking task")?;

        println!("✅ PostgreSQL connection pool created");
        Some(pool)
    } else {
        None
    };

    // SQLite pool (создаём только при необходимости)
    let sqlite_pool = if need_sqlite {
        let sqlite_path = std::env::var("SQLITE_PATH").unwrap_or_else(|_| "zid.db".to_string());

        println!("🔌 SQLite: {}", sqlite_path);

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

        println!("✅ SQLite connection pool created");
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
            println!("✅ Sessions initialized (SQLite)");
            Arc::new(repo)
        }
        "redis" => {
            let redis_client = redis::Client::open(redis_url.as_str())?;
            println!("✅ Sessions initialized (Redis)");
            Arc::new(
                adapters::persistence::redis_session::RedisSessionRepository::new(redis_client),
            )
        }
        _ => {
            // postgres (default)
            let pool = get_pg();
            let repo = adapters::persistence::postgres_session::PostgresSessionRepository::new(
                pool.clone(),
            );
            tokio::task::spawn_blocking({
                let pool = pool.clone();
                move || {
                    adapters::persistence::postgres_session::PostgresSessionRepository::new(pool)
                        .create_table()
                }
            })
            .await
            .expect("Failed to spawn blocking task")?;
            println!("✅ Sessions initialized (PostgreSQL)");
            Arc::new(repo)
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
            println!("✅ Tickets initialized (SQLite)");
            Arc::new(repo)
        }
        "redis" => {
            let redis_client = redis::Client::open(redis_url.as_str())?;
            println!("✅ Tickets initialized (Redis)");
            Arc::new(adapters::persistence::redis_ticket::RedisTicketRepository::new(redis_client))
        }
        _ => {
            let pool = get_pg();
            tokio::task::spawn_blocking({
                let pool = pool.clone();
                move || {
                    adapters::persistence::postgres_ticket::PostgresTicketRepository::new(pool)
                        .create_table()
                }
            })
            .await
            .expect("Failed to spawn blocking task")?;
            println!("✅ Tickets initialized (PostgreSQL)");
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
                println!("✅ Credentials initialized (SQLite)");
                Arc::new(repo)
            }
            "redis" => {
                let redis_client = redis::Client::open(redis_url.as_str())?;
                println!("✅ Credentials initialized (Redis)");
                Arc::new(
                    adapters::persistence::redis_credentials::RedisCredentialsRepository::new(
                        redis_client,
                    ),
                )
            }
            _ => {
                let pool = get_pg();
                tokio::task::spawn_blocking({
                let pool = pool.clone();
                move || {
                    adapters::persistence::postgres_credentials::PostgresCredentialsRepository::new(
                        pool,
                    )
                    .create_table()
                }
            })
            .await
            .expect("Failed to spawn blocking task")?;
                println!("✅ Credentials initialized (PostgreSQL)");
                Arc::new(
                    adapters::persistence::postgres_credentials::PostgresCredentialsRepository::new(
                        get_pg(),
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

    println!("✅ All repositories initialized");

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
                println!("✅ OIDC clients loaded from {}", clients_file);
                Some(Arc::new(store) as Arc<dyn ClientStore>)
            }
            Err(e) => {
                eprintln!(
                    "⚠️ OIDC disabled: failed to load clients file {}: {}",
                    clients_file, e
                );
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
                        eprintln!("⚠️ OIDC disabled: Redis required for auth codes: {}", e);
                        None
                    }
                });

        let jwt_keys = client_store.as_ref().and_then(|_| {
            let priv_path = std::path::Path::new(&private_key_path);
            let pub_path = std::path::Path::new(&public_key_path);
            match OidcJwtKeys::from_pem_paths(priv_path, pub_path, "zid-rs256-1") {
                Ok(k) => Some(Arc::new(k)),
                Err(e) => {
                    eprintln!("⚠️ OIDC disabled: failed to load JWT keys: {}", e);
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
            println!("✅ OIDC/OAuth 2.0 enabled (issuer: {})", oidc_issuer);
        } else {
            eprintln!(
                "⚠️ OIDC не включён: не хватает конфигурации (файл клиентов {}, Redis, JWT-ключи {} / {}). \
                 Запросы к /oauth/* будут возвращать 503.",
                clients_file, private_key_path, public_key_path
            );
        }
    }

    // Setup HTTP server (async handlers with sync services)
    let router = routes::create_router(router_state);

    let bind_addr = format!("{}:{}", server_host, server_port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Ошибка: не удалось привязаться к {}: {}", bind_addr, e);
            std::process::exit(1);
        }
    };

    println!();
    println!("🚀 ZID Server listening on http://{}", bind_addr);
    println!("   - Sessions storage: {}", session_storage);
    println!("   - Tickets storage: {}", ticket_storage);
    println!("   - Credentials storage: {}", credentials_storage);
    println!();

    if let Err(e) = axum::serve(listener, router).await {
        eprintln!("Ошибка: не удалось запустить сервер: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
