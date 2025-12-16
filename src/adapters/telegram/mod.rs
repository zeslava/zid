// Модуль для работы с Telegram Login Widget
// Документация: https://core.telegram.org/widgets/login

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Данные от Telegram Login Widget
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TelegramAuthData {
    pub id: i64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub auth_date: i64,
    pub hash: String,
}

impl TelegramAuthData {
    /// Проверяет подлинность данных от Telegram
    ///
    /// Алгоритм проверки согласно официальной документации:
    /// https://core.telegram.org/widgets/login#checking-authorization
    ///
    /// 1. Создаем data_check_string из всех полей (кроме hash), отсортированных по ключу
    /// 2. Вычисляем secret_key = SHA256(bot_token)
    /// 3. Вычисляем hash = HMAC-SHA256(data_check_string, secret_key)
    /// 4. Сравниваем полученный hash с hash от Telegram
    /// 5. Проверяем, что auth_date не старше 24 часов
    pub fn verify(&self, bot_token: &str) -> Result<(), String> {
        // 1. Создаем data_check_string из всех полей кроме hash
        let mut fields: Vec<(String, String)> = Vec::new();

        // Добавляем обязательные поля
        fields.push(("auth_date".to_string(), self.auth_date.to_string()));
        fields.push(("id".to_string(), self.id.to_string()));

        // Добавляем опциональные поля (только если они присутствуют)
        if let Some(ref first_name) = self.first_name {
            fields.push(("first_name".to_string(), first_name.clone()));
        }
        if let Some(ref last_name) = self.last_name {
            fields.push(("last_name".to_string(), last_name.clone()));
        }
        if let Some(ref username) = self.username {
            fields.push(("username".to_string(), username.clone()));
        }
        if let Some(ref photo_url) = self.photo_url {
            fields.push(("photo_url".to_string(), photo_url.clone()));
        }

        // 2. Сортируем поля по ключу (важно для правильной проверки!)
        fields.sort_by(|a, b| a.0.cmp(&b.0));

        // 3. Создаем data_check_string (формат: key=value\nkey=value\n...)
        let data_check_string = fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        // 4. Вычисляем secret_key = SHA256(bot_token)
        let secret_key = {
            let mut hasher = Sha256::new();
            hasher.update(bot_token.as_bytes());
            hasher.finalize()
        };

        // 5. Вычисляем HMAC-SHA256(data_check_string, secret_key)
        let mut mac = HmacSha256::new_from_slice(&secret_key)
            .map_err(|e| format!("Failed to create HMAC: {}", e))?;
        mac.update(data_check_string.as_bytes());
        let result = mac.finalize();
        let code_bytes = result.into_bytes();

        // 6. Преобразуем в hex и сравниваем с hash от Telegram
        let expected_hash = hex::encode(code_bytes);

        if expected_hash != self.hash {
            return Err(format!(
                "Hash verification failed. Expected: {}, Got: {}",
                expected_hash, self.hash
            ));
        }

        // 7. Проверяем, что auth_date не слишком старый
        // По стандарту рекомендуется не более 86400 секунд (24 часа)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let age = current_time - self.auth_date;
        if age > 86400 {
            return Err(format!(
                "Auth data is too old. Age: {} seconds (max: 86400)",
                age
            ));
        }

        if age < 0 {
            return Err(format!(
                "Auth date is in the future. This should not happen. Diff: {} seconds",
                age.abs()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_auth_data_verify() {
        // Это тестовый пример. В реальности hash будет валидным от Telegram.
        let auth_data = TelegramAuthData {
            id: 123456789,
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            username: Some("johndoe".to_string()),
            photo_url: None,
            auth_date: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            hash: "invalid_hash".to_string(),
        };

        // С неправильным токеном должна быть ошибка
        let result = auth_data.verify("fake_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_old_auth_data() {
        let old_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 90000; // 25 часов назад

        let auth_data = TelegramAuthData {
            id: 123456789,
            first_name: Some("John".to_string()),
            last_name: None,
            username: None,
            photo_url: None,
            auth_date: old_timestamp,
            hash: "some_hash".to_string(),
        };

        let result = auth_data.verify("test_token");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too old"));
    }
}
