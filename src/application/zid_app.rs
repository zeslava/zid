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
        return_to: &str,
    ) -> Result<Ticket, Error> {
        // validate return_to
        if !validate_return_to(return_to) {
            return Err(Error::InternalError(
                "Invalid return_to URL".to_string(),
            ));
        }

        // find user
        let user = self.users.get_by_username(username)?;

        // check password
        self.credentials.validate(username, password)?;

        // create session
        let session_id = uuid::Uuid::new_v4();
        let _session_id = self
            .sessions
            .create(&session_id.to_string(), user.id.as_str(), 0)?;

        // create ticket with service_url
        // Ticket TTL: 5 minutes
        let ticket_ttl = 300u64;
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + ticket_ttl;

        let ticket = self
            .tickets
            .create(&session_id.to_string(), return_to, expires_at)?;

        Ok(ticket)
    }

    fn logout(&self, session_id: &str) -> Result<(), Error> {
        self.sessions.destroy(session_id)
    }

    fn verify(
        &self,
        ticket_id: &str,
        service_url: &str,
    ) -> Result<VerificationResult, Error> {
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

const TRUSTED_DOMAINS: &[&str] = &["localhost", "127.0.0.1", "*.local.dev", "*.local", "*.lan"];

pub fn is_trusted_domain(host: &str) -> bool {
    TRUSTED_DOMAINS.iter().any(|pattern| {
        if !pattern.contains('*') {
            host == *pattern
        } else if pattern.starts_with("*.") {
            let suffix = &pattern[2..];
            host.ends_with(suffix) || host == suffix
        } else {
            false
        }
    })
}
