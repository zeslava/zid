// Бизнес-типы (User, Session, Ticket)

#[derive(Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    // Telegram fields
    pub telegram_id: Option<i64>,
    pub telegram_username: Option<String>,
    pub telegram_first_name: Option<String>,
    pub telegram_last_name: Option<String>,
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

#[derive(Clone)]
pub struct Ticket {
    pub id: String,
    pub session_id: String,
    pub service_url: String,
    pub expires_at: u64,
    pub consumed: bool,
}
