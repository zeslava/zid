// Бизнес-логика аутентификации

use std::sync::Arc;

use url::Url;

use crate::ports::{
    credentials_repository::CredentialsRepository,
    entities::{Ticket, VerificationResult},
    error::Error,
    session_repository::SessionRepository,
    ticket_repository::TicketRepository,
    user_repository::UserRepository,
    zid_service::ZidService,
};

pub struct ZidApp {
    users: Arc<dyn UserRepository>,
    sessions: Arc<dyn SessionRepository>,
    credentials: Arc<dyn CredentialsRepository>,
    tickets: Arc<dyn TicketRepository>,
}

impl ZidApp {
    pub fn new(
        users: Arc<dyn UserRepository>,
        sessions: Arc<dyn SessionRepository>,
        credentials: Arc<dyn CredentialsRepository>,
        tickets: Arc<dyn TicketRepository>,
    ) -> Self {
        ZidApp {
            users,
            sessions,
            credentials,
            tickets,
        }
    }
}

impl ZidService for ZidApp {
    fn login(
        &self,
        username: &str,
        password: &str,
        return_to: Option<&str>,
    ) -> Result<Ticket, Error> {
        // validate return_to if provided
        if let Some(url) = return_to {
            if !url.is_empty() && !validate_return_to(url) {
                return Err(Error::InternalError("Invalid return_to URL".to_string()));
            }
        }

        // find user
        let user = self.users.get_by_username(username)?;

        // check password
        self.credentials.validate(username, password)?;

        // create session (ZID SSO): 7-day expiry
        let session_id = uuid::Uuid::new_v4();
        let session_ttl_secs = 7 * 24 * 60 * 60;
        let session_expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + session_ttl_secs;
        let _session_id = self.sessions.create(
            &session_id.to_string(),
            user.id.as_str(),
            session_expires_at,
        )?;

        // create ticket with service_url
        // Ticket TTL: 5 minutes
        let ticket_ttl = 300u64;
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + ticket_ttl;

        // Use empty string if return_to is not provided
        let service_url = return_to.filter(|s| !s.is_empty()).unwrap_or("");
        let ticket = self
            .tickets
            .create(&session_id.to_string(), service_url, expires_at)?;

        Ok(ticket)
    }

    fn continue_as(&self, session_id: &str, return_to: Option<&str>) -> Result<Ticket, Error> {
        // validate return_to if provided
        if let Some(url) = return_to {
            if !url.is_empty() && !validate_return_to(url) {
                return Err(Error::InternalError("Invalid return_to URL".to_string()));
            }
        }

        // 1) Ensure session exists and isn't expired
        let session = self.sessions.get(session_id)?;

        // 2) Sliding expiration: extend by 7 days from now
        let session_ttl_secs = 7 * 24 * 60 * 60;
        let new_session_expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + session_ttl_secs;

        self.sessions.refresh(&session.id, new_session_expires_at)?;

        // 3) Issue a new one-time ticket (TTL 5 minutes)
        let ticket_ttl = 300u64;
        let ticket_expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + ticket_ttl;

        // Use empty string if return_to is not provided
        let service_url = return_to.filter(|s| !s.is_empty()).unwrap_or("");

        // Ticket must be tied to the existing session id
        let ticket = self
            .tickets
            .create(&session.id, service_url, ticket_expires_at)?;

        Ok(ticket)
    }

    fn login_telegram(
        &self,
        telegram_id: i64,
        telegram_username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        return_to: Option<&str>,
    ) -> Result<Ticket, Error> {
        // Validate return_to URL if provided
        if let Some(url) = return_to {
            if !url.is_empty() && !validate_return_to(url) {
                return Err(Error::InternalError("Invalid return_to URL".to_string()));
            }
        }

        // Проверяем, существует ли пользователь с таким Telegram ID
        let user = match self.users.get_by_telegram_id(telegram_id) {
            Ok(user) => {
                // Пользователь найден - обновляем его Telegram данные (на случай если изменились)
                let _ = self.users.update_telegram_data(
                    &user.id,
                    telegram_id,
                    telegram_username.clone(),
                    first_name.clone(),
                    last_name.clone(),
                );
                user
            }
            Err(Error::UserNotFound) => {
                // Пользователя нет - проверяем, можно ли создать нового
                let auto_register = std::env::var("TELEGRAM_AUTO_REGISTER")
                    .unwrap_or_else(|_| "true".to_string())
                    .to_lowercase();

                if auto_register == "true" || auto_register == "1" {
                    // Создаем нового пользователя
                    self.users.create_telegram_user(
                        telegram_id,
                        telegram_username,
                        first_name,
                        last_name,
                    )?
                } else {
                    return Err(Error::UserNotFound);
                }
            }
            Err(e) => return Err(e),
        };

        // Создаем сессию (ZID SSO): 7-day expiry
        let session_id = uuid::Uuid::new_v4();
        let session_ttl_secs = 7 * 24 * 60 * 60;
        let session_expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + session_ttl_secs;
        let _session_id = self.sessions.create(
            &session_id.to_string(),
            user.id.as_str(),
            session_expires_at,
        )?;

        // Создаем тикет с TTL 5 минут
        let ticket_ttl = 300u64;
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + ticket_ttl;

        // Use empty string if return_to is not provided
        let service_url = return_to.filter(|s| !s.is_empty()).unwrap_or("");
        let ticket = self
            .tickets
            .create(&session_id.to_string(), service_url, expires_at)?;

        Ok(ticket)
    }

    fn logout(&self, session_id: &str) -> Result<(), Error> {
        self.sessions.destroy(session_id)
    }

    fn verify(&self, ticket_id: &str, service_url: &str) -> Result<VerificationResult, Error> {
        // 1. Получить тикет
        let ticket = self.tickets.get(ticket_id)?;

        // 2. Проверить, что тикет не был использован
        if ticket.consumed {
            return Err(Error::TicketConsumed);
        }

        // 3. Проверить, что service_url совпадает
        if ticket.service_url != service_url {
            return Err(Error::ServiceMismatch {
                expected: ticket.service_url.clone(),
                got: service_url.to_string(),
            });
        }

        // 4. Проверить срок действия (дополнительная проверка, Redis TTL уже должен был удалить)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if ticket.expires_at > 0 && current_time > ticket.expires_at {
            // Удалить просроченный тикет
            let _ = self.tickets.delete(ticket_id);
            return Err(Error::TicketExpired);
        }

        // 5. Получить информацию о сессии
        let session = self.sessions.get(&ticket.session_id)?;

        // 6. Получить информацию о пользователе
        let user = self.users.get(&session.user_id)?;

        // 7. Удалить тикет (one-time use)
        self.tickets.delete(ticket_id)?;

        // 8. Вернуть результат верификации
        Ok(VerificationResult {
            user_id: user.id,
            username: user.username,
            session_id: session.id,
        })
    }

    fn create_user(&self, username: &str, password: &str) -> Result<(), Error> {
        // Create user in database
        self.users.create(username)?;

        // Create credentials with hashed password
        self.credentials.create_user(username, password)?;

        Ok(())
    }
}

fn validate_return_to(return_to: &str) -> bool {
    // Простая проверка, что URL начинается с доверенного домена
    let url = Url::parse(return_to);
    if url.is_err() {
        return false;
    }
    let binding = url.unwrap();
    let domain = binding.host_str();
    if domain.is_none() {
        return false;
    }

    is_trusted_domain(domain.unwrap())
}

fn get_trusted_domains() -> Vec<String> {
    // Читаем доверенные домены из переменной окружения
    // Формат: через запятую, например: "localhost,127.0.0.1,*.myapp.com,slava-pc.blue-istrian.ts.net"
    let default_domains = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "*.local.dev".to_string(),
        "*.local".to_string(),
        "*.lan".to_string(),
    ];

    match std::env::var("TRUSTED_DOMAINS") {
        Ok(val) if !val.is_empty() => val
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => default_domains,
    }
}

pub fn is_trusted_domain(host: &str) -> bool {
    let trusted_domains = get_trusted_domains();

    trusted_domains.iter().any(|pattern| {
        if !pattern.contains('*') {
            host == pattern
        } else if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            host.ends_with(suffix) || host == suffix
        } else {
            false
        }
    })
}
