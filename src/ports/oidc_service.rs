// Trait для OIDC/OAuth 2.0 сервиса

use crate::ports::{
    entities::{AuthCode, Jwks, TokenSet, UserInfo},
    error::Error,
};

pub trait OidcService: Send + Sync {
    /// Создать authorization code для последующего обмена на токены
    fn create_authorization_code(
        &self,
        client_id: &str,
        user_id: &str,
        redirect_uri: &str,
        scope: Option<&str>,
        code_challenge: Option<&str>,
        code_challenge_method: Option<&str>,
    ) -> Result<AuthCode, Error>;

    /// Обмен authorization code на токены (Authorization Code flow)
    fn exchange_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenSet, Error>;

    /// Выдать токены по client_credentials (machine-to-machine)
    fn issue_client_credentials_tokens(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<TokenSet, Error>;

    /// Валидация access_token и получение UserInfo (для userinfo endpoint)
    fn validate_access_token(&self, access_token: &str) -> Result<UserInfo, Error>;

    /// Валидация токена для userinfo: принимает access_token или id_token, возвращает UserInfo
    fn validate_userinfo_token(&self, token: &str) -> Result<UserInfo, Error>;

    /// Публичные ключи для проверки подписи JWT (JWKS)
    fn get_jwks(&self) -> Jwks;
}
