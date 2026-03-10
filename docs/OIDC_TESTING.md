# OIDC/OAuth 2.0 Testing

OIDC/OAuth 2.0 is **enabled by default**. If the client config file or JWT keys are missing at startup, the server logs a warning and starts without OIDC. To explicitly disable OIDC: `OIDC_ENABLED=false`.

**Issuer** — the authorization server (ZID) URL used by clients for discovery and JWT verification. A real domain is not required: `http://localhost:5555` works for local development; for production use a real domain with HTTPS (`https://zid.example.com`). The key requirement is that the issuer must be the address clients actually use to reach ZID. If `OIDC_ISSUER` is not set and `SERVER_HOST=0.0.0.0`, it defaults to `http://localhost:5555`.

## Setup

### 1. Generate RSA keys for JWT

```bash
# Private key (2048 bit)
openssl genrsa -out oidc_jwt_private.pem 2048

# Public key from private
openssl rsa -in oidc_jwt_private.pem -pubout -out oidc_jwt_public.pem
```

### 2. Create client config

Copy the example and edit as needed:

```bash
cp oidc_clients.example.yaml oidc_clients.yaml
```

For local testing with `redirect_uri` pointing to the same server:

```toml
[[clients]]
id = "web-app"
secret = "web-secret"
redirect_uris = ["http://localhost:5555/callback"]
grant_types = ["authorization_code"]

[[clients]]
id = "service-m2m"
secret = "machine-secret"
grant_types = ["client_credentials"]
```

### 3. Start with OIDC

Locally (PostgreSQL and Redis must be running):

```bash
export OIDC_ENABLED=true
export OIDC_ISSUER=http://localhost:5555
export OIDC_CLIENTS_FILE=oidc_clients.yaml
export OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem
export OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem
# Other variables — from .env or defaults
cargo run
```

Or as a single command:

```bash
OIDC_ENABLED=true OIDC_ISSUER=http://localhost:5555 \
  OIDC_CLIENTS_FILE=oidc_clients.yaml \
  OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem \
  OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem \
  cargo run
```

---

## Testing without a browser (curl)

Base URL for examples: `BASE=http://localhost:5555`.

### Discovery

```bash
curl -s "$BASE/.well-known/openid-configuration" | jq .
```

Expected: JSON with `issuer`, `authorization_endpoint`, `token_endpoint`, `userinfo_endpoint`, `jwks_uri`, `scopes_supported` (openid, profile, email), `grant_types_supported`.

### Client Credentials (machine-to-machine)

```bash
curl -s -X POST "$BASE/oauth/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=service-m2m" \
  -d "client_secret=machine-secret"
```

Expected: JSON with `access_token`, `token_type`, `expires_in`.

### JWKS

```bash
curl -s "$BASE/oauth/jwks" | jq .
```

Expected: JSON with `keys` (array of JWK, fields `n`, `e` for RSA).

### UserInfo (after obtaining a token)

Per OIDC/OAuth 2.0 (RFC 6750), userinfo is called with an **access_token** in the `Authorization: Bearer` header. ZID also accepts an **id_token** in the same header and a token in the `access_token` query parameter (for compatibility).

First obtain a token (client_credentials or authorization_code), then:

```bash
TOKEN="<access_token from previous step>"
curl -s "$BASE/oauth/userinfo" -H "Authorization: Bearer $TOKEN" | jq .
```

Or with a query parameter:

```bash
curl -s "$BASE/oauth/userinfo?access_token=$TOKEN" | jq .
```

**Returned claims** depend on scope and token type:

| Scope   | userinfo / id_token |
|---------|----------------------|
| —       | sub                  |
| profile | sub, name, preferred_username |
| email   | sub, name, preferred_username, email (if no stored email — `username@zid.local`) |

For client_credentials the response contains only `sub` (client_id). For authorization_code with scope openid/profile/email — the corresponding claims.

**id_token** (JWT with scope openid): contains sub, aud, exp, iat; with scope profile — name, preferred_username; with scope email — email claim. The Relying Party can extract email from the id_token without calling userinfo.

**Scopes** (in discovery `scopes_supported`): `openid` — id_token issuance; `profile` — name, preferred_username in id_token and userinfo; `email` — email claim in id_token and userinfo (value like username@zid.local if no stored email). Behavior follows OIDC Core and OAuth 2.0 Bearer Token Usage.

---

## Authorization Code flow (with browser)

1. Register a user (if not yet):
   `POST /register` with a form or via the existing `scripts/test.sh`.

2. Open the authorization URL in a browser (substitute your `redirect_uri` from `oidc_clients.yaml`):

   ```
   http://localhost:5555/oauth/authorize?response_type=code&client_id=web-app&redirect_uri=http://localhost:5555/callback&scope=openid%20profile%20email&state=random123
   ```

3. If not logged in, you'll be redirected to the login form (`/?return_to=...`). Enter username/password and submit.

4. After successful login — redirect to `redirect_uri?code=...&state=random123`. Copy the `code` value from the address bar.

5. Exchange code for tokens (substitute the actual `code` and `redirect_uri`):

   ```bash
   curl -s -X POST "http://localhost:5555/oauth/token" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code" \
     -d "client_id=web-app" \
     -d "client_secret=web-secret" \
     -d "redirect_uri=http://localhost:5555/callback" \
     -d "code=PASTE_CODE_FROM_REDIRECT"
   ```

   Response: `access_token`, `id_token` (with scope openid; with scope email the id_token will contain the email claim), `expires_in`.

6. Check UserInfo (access_token in header is recommended per OIDC):

   ```bash
   curl -s "http://localhost:5555/oauth/userinfo" \
     -H "Authorization: Bearer PASTE_ACCESS_TOKEN"
   ```

---

## Automated script (no browser)

Run:

```bash
./scripts/test-oidc.sh
```

The script checks: discovery, client_credentials, jwks, and optionally — user existence and environment. Authorization Code flow is not included in the script (requires a browser or the manual steps above).
