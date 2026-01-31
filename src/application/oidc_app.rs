// Реализация OIDC/OAuth 2.0 сервиса

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use sha2::{Digest, Sha256};

use crate::application::oidc_jwt::OidcJwtKeys;
use crate::ports::{
    auth_code_repository::AuthCodeRepository,
    client_store::ClientStore,
    entities::{AuthCode, Jwks, TokenSet, UserInfo},
    error::Error,
    oidc_service::OidcService,
    user_repository::UserRepository,
};

const AUTH_CODE_TTL_SECS: u64 = 300;

pub struct OidcApp {
    clients: Arc<dyn ClientStore>,
    auth_codes: Arc<dyn AuthCodeRepository>,
    jwt: Arc<OidcJwtKeys>,
    users: Arc<dyn UserRepository>,
    issuer: String,
}

impl OidcApp {
    pub fn new(
        clients: Arc<dyn ClientStore>,
        auth_codes: Arc<dyn AuthCodeRepository>,
        jwt: Arc<OidcJwtKeys>,
        users: Arc<dyn UserRepository>,
        issuer: String,
    ) -> Self {
        OidcApp {
            clients,
            auth_codes,
            jwt,
            users,
            issuer,
        }
    }
}

impl OidcService for OidcApp {
    fn create_authorization_code(
        &self,
        client_id: &str,
        user_id: &str,
        redirect_uri: &str,
        scope: Option<&str>,
        code_challenge: Option<&str>,
        code_challenge_method: Option<&str>,
    ) -> Result<AuthCode, Error> {
        let client = self
            .clients
            .get_client(client_id)
            .ok_or(Error::InvalidClient)?;
        if !client.grant_types.iter().any(|g| g == "authorization_code") {
            return Err(Error::UnauthorizedClient);
        }
        if !client.redirect_uris.iter().any(|u| u == redirect_uri) {
            return Err(Error::InvalidRequest(
                "redirect_uri does not match".to_string(),
            ));
        }
        let scopes: Vec<String> = scope
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();
        let code = uuid::Uuid::new_v4().to_string();
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + AUTH_CODE_TTL_SECS;
        let auth_code = AuthCode {
            code: code.clone(),
            client_id: client_id.to_string(),
            user_id: user_id.to_string(),
            redirect_uri: redirect_uri.to_string(),
            code_challenge: code_challenge.map(String::from),
            code_challenge_method: code_challenge_method.map(String::from),
            expires_at,
            scopes: scopes.clone(),
        };
        self.auth_codes.create(&auth_code, AUTH_CODE_TTL_SECS)?;
        Ok(auth_code)
    }

    fn exchange_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenSet, Error> {
        let stored = self.auth_codes.get(code)?;
        if stored.client_id != client_id {
            return Err(Error::InvalidGrant);
        }
        if stored.redirect_uri != redirect_uri {
            return Err(Error::InvalidGrant);
        }
        if let (Some(challenge), Some(verifier)) = (&stored.code_challenge, code_verifier) {
            let method = stored.code_challenge_method.as_deref().unwrap_or("plain");
            if method == "S256" {
                let computed = BASE64_URL.encode(Sha256::digest(verifier.as_bytes()));
                if computed != *challenge {
                    return Err(Error::InvalidGrant);
                }
            } else if method == "plain" && verifier != challenge {
                return Err(Error::InvalidGrant);
            }
        } else if stored.code_challenge.is_some() && code_verifier.is_none() {
            return Err(Error::InvalidGrant);
        }
        self.auth_codes.delete(code)?;

        let _client = self
            .clients
            .get_client(client_id)
            .ok_or(Error::InvalidClient)?;
        let scope_str = if stored.scopes.is_empty() {
            None
        } else {
            Some(stored.scopes.join(" "))
        };
        let wants_openid = stored.scopes.iter().any(|s| s == "openid");
        let access_token = self.jwt.sign_access_token(
            &self.issuer,
            &stored.user_id,
            client_id,
            scope_str.as_deref(),
        )?;
        let id_token = if wants_openid {
            let user = self.users.get(&stored.user_id).ok();
            let name = user.as_ref().map(|u| u.username.clone());
            let email = stored
                .scopes
                .iter()
                .any(|s| s == "email")
                .then(|| user.as_ref().map(|u| format!("{}@zid.local", u.username)))
                .flatten();
            let auth_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            Some(self.jwt.sign_id_token(
                &self.issuer,
                &stored.user_id,
                client_id,
                name.as_deref(),
                name.as_deref(),
                Some(auth_time),
                email.as_deref(),
            )?)
        } else {
            None
        };
        Ok(TokenSet {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: OidcJwtKeys::expires_in_secs(),
            refresh_token: None,
            id_token,
            scope: scope_str,
        })
    }

    fn issue_client_credentials_tokens(
        &self,
        client_id: &str,
        client_secret: &str,
    ) -> Result<TokenSet, Error> {
        let _client = self
            .clients
            .get_client(client_id)
            .ok_or(Error::InvalidClient)?;
        if !_client
            .grant_types
            .iter()
            .any(|g| g == "client_credentials")
        {
            return Err(Error::UnauthorizedClient);
        }
        match &_client.client_secret {
            Some(secret) if secret == client_secret => {}
            _ => return Err(Error::InvalidClient),
        }
        let access_token = self
            .jwt
            .sign_access_token(&self.issuer, client_id, client_id, None)?;
        Ok(TokenSet {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: OidcJwtKeys::expires_in_secs(),
            refresh_token: None,
            id_token: None,
            scope: None,
        })
    }

    fn validate_access_token(&self, access_token: &str) -> Result<UserInfo, Error> {
        let (mut info, scope) = self.jwt.verify_access_token(access_token)?;
        if let Ok(user) = self.users.get(&info.sub) {
            let username = user.username.clone();
            info.name = Some(username.clone());
            info.preferred_username = Some(username.clone());
            if scope
                .as_deref()
                .map_or(false, |s| s.split_whitespace().any(|x| x == "email"))
            {
                info.email = Some(format!("{username}@zid.local"));
            }
        }
        Ok(info)
    }

    fn validate_userinfo_token(&self, token: &str) -> Result<UserInfo, Error> {
        // Сначала пробуем как access_token
        if let Ok(info) = self.validate_access_token(token) {
            return Ok(info);
        }
        // Иначе пробуем как id_token (issuer обязателен, aud не проверяем для упрощения)
        self.jwt.verify_id_token(token, &self.issuer, None)
    }

    fn get_jwks(&self) -> Jwks {
        self.jwt.get_jwks().clone()
    }
}
