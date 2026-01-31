// Загрузка OAuth-клиентов из конфиг-файла (TOML или YAML, in-memory cache).
// Формат определяется по расширению файла в from_path (.toml / .yaml / .yml).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ports::{client_store::ClientStore, entities::OAuthClient, error::Error};

/// Хранилище клиентов из конфиг-файла (кэш в памяти). Поддерживаются форматы TOML и YAML.
pub struct FileClientStore {
    clients: HashMap<String, OAuthClient>,
}

#[derive(serde::Deserialize)]
struct ClientsFile {
    clients: Vec<ClientEntry>,
}

#[derive(serde::Deserialize)]
struct ClientEntry {
    id: String,
    #[serde(default)]
    secret: Option<String>,
    #[serde(default)]
    redirect_uris: Option<Vec<String>>,
    grant_types: Vec<String>,
}

impl FileClientStore {
    /// Загрузить клиентов из файла. Формат определяется по расширению: .toml, .yaml, .yml.
    /// Валидирует: для authorization_code обязательны redirect_uris.
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Internal(format!("OIDC clients file: {e}")))?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("toml") => Self::from_str(&content),
            Some("yaml") | Some("yml") => {
                let file: ClientsFile = serde_yaml::from_str(&content)
                    .map_err(|e| Error::Internal(format!("OIDC clients YAML: {e}")))?;
                Self::build(file)
            }
            _ => Err(Error::InvalidRequest(
                "OIDC clients file: use .toml or .yaml/.yml".to_string(),
            )),
        }
    }

    /// Загрузить из строки в формате TOML (для тестов).
    pub fn from_str(content: &str) -> Result<Self, Error> {
        let file: ClientsFile = toml::from_str(content)
            .map_err(|e| Error::Internal(format!("OIDC clients TOML: {e}")))?;
        Self::build(file)
    }

    /// Валидация и построение хранилища из распарсенного конфига.
    fn build(file: ClientsFile) -> Result<Self, Error> {
        let mut clients = HashMap::new();
        for entry in file.clients {
            if entry.grant_types.iter().any(|g| g == "authorization_code")
                && entry.redirect_uris.as_ref().is_none_or(Vec::is_empty)
            {
                return Err(Error::InvalidRequest(format!(
                    "client {}: authorization_code requires non-empty redirect_uris",
                    entry.id
                )));
            }
            let client = OAuthClient {
                client_id: entry.id.clone(),
                client_secret: entry.secret,
                redirect_uris: entry.redirect_uris.unwrap_or_default(),
                grant_types: entry.grant_types,
            };
            clients.insert(entry.id, client);
        }
        Ok(FileClientStore { clients })
    }
}

impl ClientStore for FileClientStore {
    fn get_client(&self, client_id: &str) -> Option<OAuthClient> {
        self.clients.get(client_id).cloned()
    }
}
