// Trait для хранилища OAuth-клиентов (из конфиг-файла)

use crate::ports::entities::OAuthClient;

pub trait ClientStore: Send + Sync {
    /// Получить клиента по client_id
    fn get_client(&self, client_id: &str) -> Option<OAuthClient>;
}
