// Domain error types for business logic

use std::fmt;

/// Business-level errors for the ZID CAS system
#[derive(Debug, Clone)]
pub enum Error {
    /// Authentication failed (invalid credentials, user not found, etc.)
    AuthenticationFailed,

    /// User not found in the system
    UserNotFound,

    /// Invalid credentials provided
    InvalidCredentials,

    /// Ticket not found or already consumed
    TicketNotFound,

    /// Ticket has expired
    TicketExpired,

    /// Ticket has already been consumed (one-time use)
    TicketConsumed,

    /// Service URL mismatch
    ServiceMismatch { expected: String, got: String },

    /// Session not found or invalid
    SessionNotFound,

    /// User already exists (registration conflict)
    UserAlreadyExists,

    /// Repository/storage error (database, cache, etc.)
    Repository(String),

    /// Internal error (unexpected conditions)
    Internal(String),

    // OAuth 2.0 / OIDC (RFC 6749, OIDC Core)
    /// invalid_client — клиент не найден или не прошёл аутентификацию
    InvalidClient,
    /// invalid_grant — код/токен невалиден или уже использован
    InvalidGrant,
    /// unauthorized_client — клиент не имеет права на данный grant_type
    UnauthorizedClient,
    /// invalid_scope — запрошенный scope невалиден
    #[allow(dead_code)]
    InvalidScope,
    /// invalid_request — некорректный запрос (redirect_uri и т.д.)
    InvalidRequest(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AuthenticationFailed => write!(f, "Authentication failed"),
            Error::UserNotFound => write!(f, "User not found"),
            Error::InvalidCredentials => write!(f, "Invalid credentials"),
            Error::TicketNotFound => write!(f, "Ticket not found"),
            Error::TicketExpired => write!(f, "Ticket expired"),
            Error::TicketConsumed => write!(f, "Ticket already consumed"),
            Error::ServiceMismatch { expected, got } => {
                write!(
                    f,
                    "Service URL mismatch: expected '{}', got '{}'",
                    expected, got
                )
            }
            Error::SessionNotFound => write!(f, "Session not found"),
            Error::UserAlreadyExists => write!(f, "User already exists"),
            Error::Repository(msg) => write!(f, "Repository error: {msg}"),
            Error::Internal(msg) => write!(f, "Internal error: {msg}"),
            Error::InvalidClient => write!(f, "invalid_client"),
            Error::InvalidGrant => write!(f, "invalid_grant"),
            Error::UnauthorizedClient => write!(f, "unauthorized_client"),
            Error::InvalidScope => write!(f, "invalid_scope"),
            Error::InvalidRequest(msg) => write!(f, "invalid_request: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

// Conversions for convenience
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        // Try to extract meaningful error types from anyhow
        let err_str = err.to_string();

        if err_str.contains("query returned an unexpected number of rows") {
            Error::UserNotFound
        } else if err_str.contains("duplicate key") || err_str.contains("already exists") {
            Error::UserAlreadyExists
        } else {
            Error::Repository(err_str)
        }
    }
}
