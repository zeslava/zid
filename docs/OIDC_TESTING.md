# Тестирование OIDC/OAuth 2.0

OIDC/OAuth 2.0 **включён по умолчанию**. Если при старте нет файла клиентов или JWT-ключей, сервер выведет предупреждение и запустится без OIDC. Чтобы отключить OIDC явно: `OIDC_ENABLED=false`.

**Issuer** — URL сервера авторизации (ZID), по которому клиенты обращаются к discovery и проверяют JWT. Реальный домен не обязателен: для локальной разработки достаточно `http://localhost:5555`; для продакшена обычно задают реальный домен с HTTPS (`https://zid.example.com`). Главное — issuer должен быть тем адресом, по которому клиенты реально обращаются к ZID. Если `OIDC_ISSUER` не задан, при `SERVER_HOST=0.0.0.0` подставляется `http://localhost:5555`.

## Подготовка

### 1. Сгенерировать RSA-ключи для JWT

```bash
# Приватный ключ (2048 бит)
openssl genrsa -out oidc_jwt_private.pem 2048

# Публичный ключ из приватного
openssl rsa -in oidc_jwt_private.pem -pubout -out oidc_jwt_public.pem
```

### 2. Создать конфиг клиентов

Скопировать пример и при необходимости отредактировать:

```bash
cp oidc_clients.example.yaml oidc_clients.yaml
```

Для локального теста с `redirect_uri` на этот же сервер можно указать:

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

### 3. Запуск с OIDC

Локально (PostgreSQL и Redis должны быть запущены):

```bash
export OIDC_ENABLED=true
export OIDC_ISSUER=http://localhost:5555
export OIDC_CLIENTS_FILE=oidc_clients.yaml
export OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem
export OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem
# Остальные переменные — по .env или дефолтам
cargo run
```

Или через один вызов:

```bash
OIDC_ENABLED=true OIDC_ISSUER=http://localhost:5555 \
  OIDC_CLIENTS_FILE=oidc_clients.yaml \
  OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem \
  OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem \
  cargo run
```

---

## Проверка без браузера (curl)

Базовый URL для примеров: `BASE=http://localhost:5555`.

### Discovery

```bash
curl -s "$BASE/.well-known/openid-configuration" | jq .
```

Ожидается JSON с `issuer`, `authorization_endpoint`, `token_endpoint`, `userinfo_endpoint`, `jwks_uri`, `scopes_supported` (openid, profile, email), `grant_types_supported`.

### Client Credentials (machine-to-machine)

```bash
curl -s -X POST "$BASE/oauth/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=service-m2m" \
  -d "client_secret=machine-secret"
```

Ожидается JSON с `access_token`, `token_type`, `expires_in`.

### JWKS

```bash
curl -s "$BASE/oauth/jwks" | jq .
```

Ожидается JSON с `keys` (массив JWK, поле `n`, `e` для RSA).

### UserInfo (после получения токена)

По OIDC/OAuth 2.0 (RFC 6750) userinfo вызывается с **access_token** в заголовке `Authorization: Bearer`. ZID также принимает **id_token** в том же заголовке и токен в query-параметре `access_token` (для совместимости).

Сначала получить токен (client_credentials или authorization_code), затем:

```bash
TOKEN="<access_token из предыдущего шага>"
curl -s "$BASE/oauth/userinfo" -H "Authorization: Bearer $TOKEN" | jq .
```

Или с query-параметром:

```bash
curl -s "$BASE/oauth/userinfo?access_token=$TOKEN" | jq .
```

**Возвращаемые claims** зависят от scope и типа токена:

| Scope   | userinfo / id_token |
|---------|----------------------|
| —       | sub                  |
| profile | sub, name, preferred_username |
| email   | sub, name, preferred_username, email (при отсутствии хранимого email — `username@zid.local`) |

Для client_credentials в ответе будет только `sub` (client_id). Для authorization_code при scope openid/profile/email — соответствующие claims.

**id_token** (JWT при scope openid): содержит sub, aud, exp, iat, при scope profile — name, preferred_username, при scope email — claim email. Relying Party может брать email из id_token без вызова userinfo.

**Scopes** (в discovery `scopes_supported`): `openid` — выдача id_token; `profile` — name, preferred_username в id_token и userinfo; `email` — claim email в id_token и в userinfo (значение типа username@zid.local при отсутствии хранимого email). Поведение соответствует OIDC Core и OAuth 2.0 Bearer Token Usage.

---

## Authorization Code flow (с браузером)

1. Зарегистрировать пользователя (если ещё нет):  
   `POST /register` с формой или через существующий `scripts/test.sh`.

2. Открыть в браузере URL авторизации (подставьте свой `redirect_uri` из `oidc_clients.yaml`):

   ```
   http://localhost:5555/oauth/authorize?response_type=code&client_id=web-app&redirect_uri=http://localhost:5555/callback&scope=openid%20profile%20email&state=random123
   ```

3. Если вы не залогинены, произойдёт редирект на форму логина (`/?return_to=...`). Введите логин/пароль и отправьте форму.

4. После успешного входа — редирект на `redirect_uri?code=...&state=random123`. Скопируйте значение `code` из адресной строки.

5. Обмен code на токены (подставьте реальные `code` и `redirect_uri`):

   ```bash
   curl -s -X POST "http://localhost:5555/oauth/token" \
     -H "Content-Type: application/x-www-form-urlencoded" \
     -d "grant_type=authorization_code" \
     -d "client_id=web-app" \
     -d "client_secret=web-secret" \
     -d "redirect_uri=http://localhost:5555/callback" \
     -d "code=ВСТАВЬТЕ_КОД_ИЗ_РЕДИРЕКТА"
   ```

   Ответ: `access_token`, `id_token` (при scope openid; при scope email в id_token будет claim email), `expires_in`.

6. Проверить UserInfo (рекомендуется access_token в заголовке, по OIDC):

   ```bash
   curl -s "http://localhost:5555/oauth/userinfo" \
     -H "Authorization: Bearer ВСТАВЬТЕ_ACCESS_TOKEN"
   ```

---

## Автоматический скрипт (без браузера)

Запуск:

```bash
./scripts/test-oidc.sh
```

Скрипт проверяет: discovery, client_credentials, jwks, при необходимости — наличие пользователя и окружения. Authorization Code flow в скрипт не входит (нужен браузер или ручные шаги выше).
