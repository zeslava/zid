// CLI: разбор аргументов командной строки и управление OIDC-клиентами.

use std::path::Path;

use clap::{Parser, Subcommand};
use inquire::{MultiSelect, Text, validator::Validation};

use crate::adapters::oidc::file_client_store::{ClientEntry, ClientsFile};

#[derive(Parser)]
#[command(name = "zid", about = "ZID — CAS-like SSO authentication server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Запуск сервера
    Serve,
    /// Управление OIDC клиентами
    OidcClient {
        #[command(subcommand)]
        action: OidcClientAction,
    },
}

#[derive(Subcommand)]
pub enum OidcClientAction {
    /// Список зарегистрированных клиентов
    List,
    /// Добавить нового клиента (без флагов — интерактивный режим)
    Add {
        /// Идентификатор клиента
        #[arg(long)]
        id: Option<String>,
        /// Секрет клиента (опциональный)
        #[arg(long)]
        secret: Option<String>,
        /// URI для редиректа (можно указать несколько раз)
        #[arg(long = "redirect-uri")]
        redirect_uris: Vec<String>,
        /// Разрешённые grant types (можно указать несколько раз)
        #[arg(long = "grant-type")]
        grant_types: Vec<String>,
    },
    /// Удалить клиента по id
    Remove {
        /// Идентификатор клиента
        id: String,
    },
}

/// Точка входа для CLI-команд oidc-client.
pub fn handle_oidc_client(action: OidcClientAction) {
    let clients_file =
        std::env::var("OIDC_CLIENTS_FILE").unwrap_or_else(|_| "oidc_clients.yaml".to_string());
    let path = Path::new(&clients_file);

    match action {
        OidcClientAction::List => cmd_list(path),
        OidcClientAction::Add {
            id,
            secret,
            redirect_uris,
            grant_types,
        } => cmd_add(path, id, secret, redirect_uris, grant_types),
        OidcClientAction::Remove { id } => cmd_remove(path, &id),
    }
}

fn load_clients_file(path: &Path) -> Option<ClientsFile> {
    if !path.exists() {
        eprintln!("Файл клиентов не найден: {}", path.display());
        return None;
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Не удалось прочитать {}: {e}", path.display());
            return None;
        }
    };
    match serde_yaml::from_str::<ClientsFile>(&content) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("Ошибка парсинга YAML {}: {e}", path.display());
            None
        }
    }
}

fn save_clients_file(path: &Path, file: &ClientsFile) {
    let yaml = match serde_yaml::to_string(file) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("Ошибка сериализации YAML: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = std::fs::write(path, yaml) {
        eprintln!("Не удалось записать {}: {e}", path.display());
        std::process::exit(1);
    }
}

fn cmd_list(path: &Path) {
    let Some(file) = load_clients_file(path) else {
        return;
    };
    if file.clients.is_empty() {
        println!("Нет зарегистрированных клиентов.");
        return;
    }
    println!(
        "{:<20} {:<20} {:<30} {}",
        "ID", "SECRET", "GRANT TYPES", "REDIRECT URIs"
    );
    println!("{}", "-".repeat(90));
    for c in &file.clients {
        let secret_display = match &c.secret {
            Some(s) => {
                if s.len() > 4 {
                    format!("{}****", &s[..4])
                } else {
                    "****".to_string()
                }
            }
            None => "-".to_string(),
        };
        let grants = c.grant_types.join(", ");
        let uris = c
            .redirect_uris
            .as_ref()
            .map(|u| u.join(", "))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<20} {:<20} {:<30} {}",
            c.id, secret_display, grants, uris
        );
    }
}

/// Интерактивный ввод данных клиента через промпты.
fn prompt_client() -> (String, Option<String>, Vec<String>, Vec<String>) {
    let id = Text::new("Client ID:")
        .with_validator(|val: &str| {
            if val.trim().is_empty() {
                Ok(Validation::Invalid("Client ID не может быть пустым".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .unwrap_or_else(|e| {
            eprintln!("Ошибка ввода: {e}");
            std::process::exit(1);
        });

    let secret_input = Text::new("Client Secret (пустой = без секрета):")
        .prompt()
        .unwrap_or_else(|e| {
            eprintln!("Ошибка ввода: {e}");
            std::process::exit(1);
        });
    let secret = if secret_input.trim().is_empty() {
        None
    } else {
        Some(secret_input)
    };

    let grant_options = vec!["authorization_code", "client_credentials", "refresh_token"];
    let grant_types = MultiSelect::new("Grant Types:", grant_options)
        .with_validator(|selected: &[inquire::list_option::ListOption<&&str>]| {
            if selected.is_empty() {
                Ok(Validation::Invalid(
                    "Выберите хотя бы один grant type".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .unwrap_or_else(|e| {
            eprintln!("Ошибка ввода: {e}");
            std::process::exit(1);
        })
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    let mut redirect_uris = Vec::new();
    if grant_types.iter().any(|g| g == "authorization_code") {
        println!("Введите Redirect URIs (пустая строка = конец):");
        loop {
            let uri = Text::new("Redirect URI:").prompt().unwrap_or_else(|e| {
                eprintln!("Ошибка ввода: {e}");
                std::process::exit(1);
            });
            if uri.trim().is_empty() {
                break;
            }
            redirect_uris.push(uri);
        }
        if redirect_uris.is_empty() {
            eprintln!("Ошибка: authorization_code требует хотя бы один redirect URI");
            std::process::exit(1);
        }
    }

    (id, secret, redirect_uris, grant_types)
}

fn cmd_add(
    path: &Path,
    id: Option<String>,
    secret: Option<String>,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
) {
    let (id, secret, redirect_uris, grant_types) = if id.is_none() || grant_types.is_empty() {
        prompt_client()
    } else {
        (id.unwrap(), secret, redirect_uris, grant_types)
    };

    if grant_types.is_empty() {
        eprintln!("Ошибка: требуется хотя бы один --grant-type");
        std::process::exit(1);
    }

    if grant_types.iter().any(|g| g == "authorization_code") && redirect_uris.is_empty() {
        eprintln!("Ошибка: grant type authorization_code требует хотя бы один --redirect-uri");
        std::process::exit(1);
    }

    let mut file = if path.exists() {
        load_clients_file(path).unwrap_or(ClientsFile {
            clients: Vec::new(),
        })
    } else {
        ClientsFile {
            clients: Vec::new(),
        }
    };

    if file.clients.iter().any(|c| c.id == id) {
        eprintln!("Ошибка: клиент с id '{id}' уже существует");
        std::process::exit(1);
    }

    let entry = ClientEntry {
        id: id.clone(),
        secret,
        redirect_uris: if redirect_uris.is_empty() {
            None
        } else {
            Some(redirect_uris)
        },
        grant_types,
    };

    file.clients.push(entry);
    save_clients_file(path, &file);
    println!("Клиент '{id}' добавлен.");
}

fn cmd_remove(path: &Path, id: &str) {
    let Some(mut file) = load_clients_file(path) else {
        return;
    };
    let before = file.clients.len();
    file.clients.retain(|c| c.id != id);
    if file.clients.len() == before {
        eprintln!("Ошибка: клиент с id '{id}' не найден");
        std::process::exit(1);
    }
    save_clients_file(path, &file);
    println!("Клиент '{id}' удалён.");
}
