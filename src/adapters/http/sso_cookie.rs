//! Helpers for ZID SSO cookie management.
//!
//! This module is intentionally small and dependency-light so it can be used
//! from HTTP handlers without pulling extra middleware.
//!
//! Design goals:
//! - One cookie that stores the ZID session id (UUID string)
//! - Secure defaults for production (Https + Secure cookie)
//! - Sliding expiration support (extend expiry on each recognized request)
//! - Minimal parsing/formatting helpers for `Set-Cookie` and `Cookie` headers
//!
//! Notes:
//! - We rely only on `axum::http::HeaderMap` for reading request headers.
//! - For setting cookies, we return the value to be used in the `Set-Cookie` header.

use axum::http::HeaderMap;

/// Name of the cookie that holds the ZID SSO session id.
pub const ZID_SSO_COOKIE_NAME: &str = "zid_sso";

/// Default SSO session duration: 7 days.
pub const DEFAULT_SSO_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Environment variable controlling the `Secure` cookie attribute.
///
/// Values:
/// - `auto` (default): infer from request headers (X-Forwarded-Proto, Forwarded, etc)
/// - `true` / `1`: always set `Secure`
/// - `false` / `0`: never set `Secure` (useful for local HTTP dev)
pub const COOKIE_SECURE_ENV: &str = "ZID_COOKIE_SECURE";

/// Default `SameSite` attribute.
pub const DEFAULT_SAMESITE: SameSite = SameSite::Lax;

/// Default cookie path.
pub const DEFAULT_COOKIE_PATH: &str = "/";

/// Cookie SameSite setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SameSite {
    Lax,
    Strict,
    None,
}

impl SameSite {
    pub fn as_str(self) -> &'static str {
        match self {
            SameSite::Lax => "Lax",
            SameSite::Strict => "Strict",
            SameSite::None => "None",
        }
    }
}

/// Configuration for building a Set-Cookie header for ZID SSO.
///
/// You usually want:
/// - `http_only = true`
/// - `secure = true` when behind HTTPS
/// - `same_site = Lax` (works well with normal navigation/redirects)
/// - `ttl_secs = 7 days`
///
/// `domain` is optional; most deployments should *not* set it so the cookie stays
/// scoped to the ZID host.
#[derive(Debug, Clone)]
pub struct SsoCookieConfig {
    pub name: &'static str,
    pub path: &'static str,
    pub domain: Option<String>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub ttl_secs: u64,
}

impl Default for SsoCookieConfig {
    fn default() -> Self {
        Self {
            name: ZID_SSO_COOKIE_NAME,
            path: DEFAULT_COOKIE_PATH,
            domain: None,
            secure: true,
            http_only: true,
            same_site: DEFAULT_SAMESITE,
            ttl_secs: DEFAULT_SSO_TTL_SECS,
        }
    }
}

/// Extracts `zid_sso` session id from incoming request headers.
///
/// Returns `None` if:
/// - `Cookie` header is missing
/// - cookie name isn't present
/// - cookie value is empty
///
/// Note: this parser is intentionally minimal; it splits `Cookie` header on `;`
/// and then `=`. It does not handle quoted values (not needed for UUIDs).
pub fn get_sso_session_id(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;

    for part in cookie_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // "name=value"
        let mut it = part.splitn(2, '=');
        let name = it.next()?.trim();
        let value = it.next().unwrap_or("").trim();
        if name == ZID_SSO_COOKIE_NAME && !value.is_empty() {
            return Some(value.to_string());
        }
    }

    None
}

/// Builds a `Set-Cookie` header value for setting the SSO cookie.
///
/// `session_id` should be a UUID string (we don't validate here).
///
/// This uses `Max-Age` (in seconds) for portability.
pub fn build_set_cookie(session_id: &str, cfg: &SsoCookieConfig) -> String {
    // Base: name=value
    let mut out = String::new();
    out.push_str(cfg.name);
    out.push('=');
    out.push_str(session_id);

    // Path
    out.push_str("; Path=");
    out.push_str(cfg.path);

    // Domain (optional)
    if let Some(domain) = &cfg.domain
        && !domain.is_empty()
    {
        out.push_str("; Domain=");
        out.push_str(domain);
    }

    // Expiration (sliding: handlers should call this again when needed)
    out.push_str("; Max-Age=");
    out.push_str(&cfg.ttl_secs.to_string());

    // SameSite
    out.push_str("; SameSite=");
    out.push_str(cfg.same_site.as_str());

    // Secure / HttpOnly
    if cfg.secure {
        out.push_str("; Secure");
    }
    if cfg.http_only {
        out.push_str("; HttpOnly");
    }

    out
}

/// Builds a `Set-Cookie` header value that clears the SSO cookie on the client.
///
/// Convention:
/// - set empty value
/// - set `Max-Age=0`
/// - keep Path stable
pub fn build_clear_cookie(cfg: &SsoCookieConfig) -> String {
    let mut out = String::new();
    out.push_str(cfg.name);
    out.push_str("=; Path=");
    out.push_str(cfg.path);
    out.push_str("; Max-Age=0");
    out.push_str("; SameSite=");
    out.push_str(cfg.same_site.as_str());

    if cfg.secure {
        out.push_str("; Secure");
    }
    if cfg.http_only {
        out.push_str("; HttpOnly");
    }

    out
}

/// Returns `true` when the request is considered HTTPS.
///
/// This is best-effort and intentionally conservative:
/// - Prefer `X-Forwarded-Proto: https` (common behind reverse proxies)
/// - Support RFC 7239 `Forwarded: proto=https`
/// - Otherwise assume non-https
///
/// In production behind HTTPS termination you should configure your proxy to pass
/// `X-Forwarded-Proto=https`.
pub fn is_https_request(headers: &HeaderMap) -> bool {
    // 1) X-Forwarded-Proto
    if let Some(v) = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        && v.eq_ignore_ascii_case("https")
    {
        return true;
    }

    // 2) Forwarded: proto=https; ...
    if let Some(v) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        // Extremely small parser: look for "proto=https" token
        // (case-insensitive, allows spaces and semicolons/commas).
        let lower = v.to_ascii_lowercase();
        if lower.contains("proto=https") {
            return true;
        }
    }

    false
}

/// Convenience: create a config using defaults but with `secure` determined from env + request.
///
/// Rule:
/// - If `ZID_COOKIE_SECURE` is set to `true/1`  -> always Secure
/// - If `ZID_COOKIE_SECURE` is set to `false/0` -> never Secure (local HTTP)
/// - Otherwise (default `auto`) -> infer from request headers (`is_https_request`)
pub fn default_config_for_request(headers: &HeaderMap) -> SsoCookieConfig {
    SsoCookieConfig {
        secure: cookie_secure_effective(headers),
        ..Default::default()
    }
}

/// Computes the effective `Secure` attribute for the cookie.
pub fn cookie_secure_effective(headers: &HeaderMap) -> bool {
    match std::env::var(COOKIE_SECURE_ENV)
        .unwrap_or_else(|_| "auto".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => is_https_request(headers),
    }
}
