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

// --- OIDC/OAuth 2.0 ---

/// OAuth-клиент (из конфига)
#[derive(Clone)]
pub struct OAuthClient {
    #[allow(dead_code)] // ключ при загрузке из конфига, может понадобиться в API
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
}

/// Одноразовый authorization code (Authorization Code flow)
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthCode {
    pub code: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: u64,
    pub scopes: Vec<String>,
}

/// Набор токенов (ответ token endpoint)
#[derive(Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scope: Option<String>,
}

/// Стандартные claims UserInfo (OIDC)
#[derive(Clone, serde::Serialize)]
pub struct UserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub email: Option<String>,
}

/// JWKS (JSON Web Key Set) для проверки подписи JWT
#[derive(Clone, serde::Serialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

#[derive(Clone, serde::Serialize)]
pub struct Jwk {
    pub kty: String,
    pub kid: String,
    pub r#use: String,
    pub alg: String,
    pub n: String,
    pub e: String,
}
