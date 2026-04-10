// Загрузка OAuth-клиентов из конфиг-файла YAML (in-memory cache).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::ports::{client_store::ClientStore, entities::OAuthClient, error::Error};

/// Хранилище клиентов из конфиг-файла (кэш в памяти). Формат: YAML (.yaml / .yml).
pub struct FileClientStore {
    clients: HashMap<String, OAuthClient>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ClientsFile {
    pub clients: Vec<ClientEntry>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ClientEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uris: Option<Vec<String>>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".to_string()]
}

impl FileClientStore {
    /// Загрузить клиентов из файла YAML (.yaml / .yml).
    /// Валидирует: для authorization_code обязательны redirect_uris.
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Internal(format!("OIDC clients file: {e}")))?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        if ext.as_deref() != Some("yaml") && ext.as_deref() != Some("yml") {
            return Err(Error::InvalidRequest(
                "OIDC clients file: use .yaml or .yml".to_string(),
            ));
        }
        Self::from_str(&content)
    }

    /// Загрузить из строки в формате YAML (для тестов).
    pub fn from_str(content: &str) -> Result<Self, Error> {
        let file: ClientsFile = serde_yaml::from_str(content)
            .map_err(|e| Error::Internal(format!("OIDC clients YAML: {e}")))?;
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
