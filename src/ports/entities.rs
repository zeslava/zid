// Бизнес-типы (User, Session, Ticket)

pub struct User {
    pub id: String,
    pub username: String,
}

/// Результат верификации тикета
pub struct VerificationResult {
    pub user_id: String,
    pub username: String,
    pub session_id: String,
}

pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: u64,
}

impl Session {
    pub fn new(id: String, user_id: String, expires_at: u64) -> Self {
        Session {
            id,
            user_id,
            expires_at,
        }
    }
}

#[derive(Clone)]
pub struct Ticket {
    pub id: String,
    pub session_id: String,
    pub service_url: String,
    pub expires_at: u64,
    pub consumed: bool,
}

impl Ticket {
    pub fn new(id: String, session_id: String, service_url: String, expires_at: u64) -> Self {
        Ticket {
            id,
            session_id,
            service_url,
            expires_at,
            consumed: false,
        }
    }
}
