// JWT подпись/верификация и JWKS для OIDC

use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rsa::RsaPublicKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};

use crate::ports::entities::{Jwk, Jwks, UserInfo};

const KID: &str = "zid-rs256-1";
const AUTH_CODE_TTL_SECS: i64 = 3600; // 1 час для access_token

/// Загрузка ключей и подпись JWT
pub struct OidcJwtKeys {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    jwks: Jwks,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub iss: String,
    pub sub: String,
    pub client_id: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl OidcJwtKeys {
    /// Загрузить ключи из PEM-файлов (private — для подписи, public — для верификации и JWKS)
    pub fn from_pem_paths(
        private_key_path: &Path,
        public_key_path: &Path,
        kid: &str,
    ) -> Result<Self, crate::ports::error::Error> {
        let private_pem = fs::read_to_string(private_key_path)
            .map_err(|e| crate::ports::error::Error::Internal(format!("OIDC private key: {e}")))?;
        let public_pem = fs::read_to_string(public_key_path)
            .map_err(|e| crate::ports::error::Error::Internal(format!("OIDC public key: {e}")))?;

        let encoding_key = EncodingKey::from_rsa_pem(private_pem.as_bytes())
            .map_err(|e| crate::ports::error::Error::Internal(format!("OIDC encoding key: {e}")))?;
        let decoding_key = DecodingKey::from_rsa_pem(public_pem.as_bytes())
            .map_err(|e| crate::ports::error::Error::Internal(format!("OIDC decoding key: {e}")))?;

        let jwks = build_jwks_from_public_pem(&public_pem, kid)?;

        Ok(OidcJwtKeys {
            encoding_key,
            decoding_key,
            jwks,
        })
    }

    /// Подписать id_token (OIDC)
    pub fn sign_id_token(
        &self,
        iss: &str,
        sub: &str,
        aud: &str,
        name: Option<&str>,
        preferred_username: Option<&str>,
        auth_time: Option<i64>,
        email: Option<&str>,
    ) -> Result<String, crate::ports::error::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let claims = IdTokenClaims {
            iss: iss.to_string(),
            sub: sub.to_string(),
            aud: aud.to_string(),
            exp: now + AUTH_CODE_TTL_SECS,
            iat: now,
            auth_time,
            name: name.map(String::from),
            preferred_username: preferred_username.map(String::from),
            email: email.map(String::from),
        };
        let header = Header {
            alg: jsonwebtoken::Algorithm::RS256,
            kid: Some(KID.to_string()),
            ..Default::default()
        };
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| crate::ports::error::Error::Internal(format!("JWT encode: {e}")))
    }

    /// Подписать access_token (JWT)
    pub fn sign_access_token(
        &self,
        iss: &str,
        sub: &str,
        client_id: &str,
        scope: Option<&str>,
    ) -> Result<String, crate::ports::error::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let claims = AccessTokenClaims {
            iss: iss.to_string(),
            sub: sub.to_string(),
            client_id: client_id.to_string(),
            exp: now + AUTH_CODE_TTL_SECS,
            iat: now,
            scope: scope.map(String::from),
        };
        let header = Header {
            alg: jsonwebtoken::Algorithm::RS256,
            kid: Some(KID.to_string()),
            ..Default::default()
        };
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| crate::ports::error::Error::Internal(format!("JWT encode: {e}")))
    }

    /// Верифицировать access_token и извлечь claims для UserInfo; возвращает UserInfo и scope для подстановки email.
    pub fn verify_access_token(
        &self,
        token: &str,
    ) -> Result<(UserInfo, Option<String>), crate::ports::error::Error> {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.validate_exp = true;
        validation.set_issuer::<&str>(&[]);
        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| crate::ports::error::Error::InvalidGrant)?;
        let info = UserInfo {
            sub: token_data.claims.sub,
            name: None,
            preferred_username: None,
            email: None,
        };
        Ok((info, token_data.claims.scope))
    }

    /// Верифицировать id_token и извлечь UserInfo (для userinfo endpoint при передаче id_token).
    pub fn verify_id_token(
        &self,
        token: &str,
        expected_iss: &str,
        expected_aud: Option<&str>,
    ) -> Result<UserInfo, crate::ports::error::Error> {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.validate_exp = true;
        validation.set_issuer(&[expected_iss]);
        if let Some(aud) = expected_aud {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }
        let token_data = decode::<IdTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|_| crate::ports::error::Error::InvalidGrant)?;
        Ok(UserInfo {
            sub: token_data.claims.sub,
            name: token_data.claims.name,
            preferred_username: token_data.claims.preferred_username,
            email: token_data.claims.email,
        })
    }

    pub fn get_jwks(&self) -> &Jwks {
        &self.jwks
    }

    pub fn expires_in_secs() -> u64 {
        AUTH_CODE_TTL_SECS as u64
    }
}

/// Построить JWKS из PEM публичного ключа (SPKI)
fn build_jwks_from_public_pem(pem: &str, kid: &str) -> Result<Jwks, crate::ports::error::Error> {
    let pub_key = RsaPublicKey::from_public_key_pem(pem)
        .map_err(|e| crate::ports::error::Error::Internal(format!("JWKS from PEM: {e}")))?;
    let n_bytes = pub_key.n().to_bytes_be();
    let e_bytes = pub_key.e().to_bytes_be();
    let n_b64 = BASE64_URL.encode(&n_bytes);
    let e_b64 = BASE64_URL.encode(&e_bytes);
    Ok(Jwks {
        keys: vec![Jwk {
            kty: "RSA".to_string(),
            kid: kid.to_string(),
            r#use: "sig".to_string(),
            alg: "RS256".to_string(),
            n: n_b64,
            e: e_b64,
        }],
    })
}
