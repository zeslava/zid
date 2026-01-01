# ZID Integration Guide

ZID — CAS-подобный сервер аутентификации.

**Принцип:**
1. Пользователь логинится в ZID
2. ZID выдаёт **one-time ticket**
3. Верифицируете ticket через `/verify` → получаете `user_id`, `username`
4. Создаёте свою сессию

## Интеграция

### 1. Браузер + Редирект (основной способ)

**Ссылка логина:**
```html
<a href="http://zid-host:5555/?return_to=https://yourapp.com/callback">
  Войти
</a>
```

**Callback-обработчик (GET /callback?ticket=xxx):**
```rust
use serde_json::json;

#[derive(serde::Deserialize)]
struct VerResult {
    user_id: String,
    username: String,
}

async fn callback(Query(q): Query<std::collections::HashMap<String, String>>) -> Result<Redirect> {
    let ticket = q.get("ticket").ok_or("No ticket")?;
    
    let client = reqwest::Client::new();
    let res = client
        .post("http://zid-host:5555/verify")
        .json(&json!({
            "ticket": ticket,
            "service": "https://yourapp.com/callback"
        }))
        .send()
        .await?;
    
    let result: VerResult = res.json().await?;
    
    // Создать свою сессию с result.user_id
    create_session(result.user_id).await?;
    
    Ok(Redirect::to("/dashboard"))
}
```

**Важно:** Добавьте домен в `TRUSTED_DOMAINS` конфигурации ZID.

---

### Вариант 2: JSON API (без редиректа)

Для мобильных приложений или SPA, которые работают напрямую с API.

#### Шаг 1: Отправить учётные данные на ZID

```bash
curl -X POST http://zid-host:5555/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "john",
    "password": "secret"
  }'
```

Ответ:
```json
{
  "ticket": "550e8400-e29b-41d4-a716-446655440000",
  "redirect_url": null
}
```

#### Шаг 2: Верифицировать ticket

```rust
#[derive(serde::Deserialize)]
struct LoginResponse {
    ticket: String,
    redirect_url: Option<String>,
}

async fn login_api(username: &str, password: &str) -> Result<LoginResponse> {
    let client = reqwest::Client::new();
    
    let response = client
        .post("http://zid-host:5555/login")
        .json(&json!({
            "username": username,
            "password": password
        }))
        .send()
        .await?;
    
    Ok(response.json().await?)
}

async fn handle_login_response(ticket: String) -> Result<()> {
    let client = reqwest::Client::new();
    
    let verification = client
        .post("http://zid-host:5555/verify")
        .json(&json!({
            "ticket": ticket,
            "service": "my-api-service"  // уникальный идентификатор вашего сервиса
        }))
        .send()
        .await?
        .json::<VerificationResult>()
        .await?;
    
    // Создайте свою сессию с verification.user_id
    
    Ok(())
}
```

---

### Вариант 3: Telegram Login

ZID поддерживает логин через Telegram.

```bash
curl -X POST http://zid-host:5555/login/telegram \
  -H "Content-Type: application/json" \
  -d '{
    "telegram_id": 123456789,
    "telegram_username": "johndoe",
    "first_name": "John",
    "last_name": "Doe",
    "return_to": "https://yourapp.com/callback"
  }'
```

Ответ:
```json
{
  "ticket": "550e8400-e29b-41d4-a716-446655440000",
  "redirect_url": "https://yourapp.com/callback?ticket=..."
}
```

### 2. JSON API (для SPA/мобильных)

```rust
// POST /login
let res = client
    .post("http://zid-host:5555/login")
    .json(&json!({"username": "john", "password": "secret"}))
    .send()
    .await?;

let login_res: serde_json::Value = res.json().await?;
let ticket = login_res["ticket"].as_str().unwrap();

// Верифицировать ticket
let res = client
    .post("http://zid-host:5555/verify")
    .json(&json!({"ticket": ticket, "service": "my-service"}))
    .send()
    .await?;

let user = res.json::<VerResult>().await?;
// Создать сессию с user.user_id
```

TGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=zid
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres

# Redis (для сессий и ticket'ов)
REDIS_URL=redis://127.0.0.1:6380

# Сервер
SERVER_HOST=0.0.0.0
SERVER_PORT=5555

# Trusted domains (comma-separated, поддерживает wildcards)
TRUSTED_DOMAINS=localhost,127.0.0.1,*.local.dev,*.local,*.lan

# Хранилища (postgres или redis)
SESSION_STORAGE=redis
TICKET_STORAGE=redis
CREDENTIALS_STORAGE=postgres

# Cookie security
# auto - определяет по заголовкам (X-Forwarded-Proto)
# true - всегда Secure
# false - никогда Secure (для локальной разработки по HTTP)
ZID_COOKIE_SECURE=false

# Telegram
TELEGRAM_AUTO_REGISTER=true
```

### Важно: Cookie Security для локальной разработки

Если вы разрабатываете локально по HTTP (`http://localhost`):

```bash
# В .env или docker-compose.yml
ZID_COOKIE_SECURE=false
```

Иначе браузер не будет сохранять cookie `zid_sso` (современные браузеры требуют `Secure` для HTTPS).

Для продакшена (HTTPS):

```bash
ZID_COOKIE_SECURE=auto  # или true
```

Убедитесь, что прокси (nginx, etc.) прокидывает заголовок:
```
X-Forwarded-Proto: https
```

---

## Типичные ошибки

### 1. "Invalid return_to URL"

**Проблема:** Домен не в `TRUSTED_DOMAINS`.

**Решение:** Добавьте домен в конфигурацию ZID:
```bash
TRUSTED_DOMAINS=localhost,127.0.0.1,*.yourapp.com,yourapp.com
```

### 2. SSO cookie не сохраняется локально

**Проблема:** `ZID_COOKIE_SECURE=true` при `http://localhost`.

**Решение:** Используйте `ZID_COOKIE_SECURE=false` для локальной разработки.

### 3. Ticket уже использован

**Проблема:** Попытка верифицировать один ticket дважды.

**Решение:** Ticket — one-time, используется только один раз. При повторной попытке требуется новый ticket.

### 4. CORS/Cookie не отправляются в запросе

**Проблема:** Запрос не отправляет cookie.

**Решение:** Используйте `credentials: 'include'` в fetch:
```javascript
fetch('http://zid-host:5555/verify', {
  method: 'POST',
  credentials: 'include',  // Важно!
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ ticket, service })
})
```

---

## Примеры реализации

### Axum + Redirect Flow

```rust
use axum::{
## Конфигурация

Переменные в ZID:
```bash
SERVER_HOST=0.0.0.0
SERVER_PORT=5555
POSTGRES_HOST=localhost
REDIS_URL=redis://127.0.0.1:6380
TRUSTED_DOMAINS=localhost,127.0.0.1,*.yourapp.com
ZID_COOKIE_SECURE=false  # для локальной разработки (HTTP)
```

**Важно:** Добавьте домен вашего приложения в `TRUSTED_DOMAINS`.

## Типичные ошибки

- **"Invalid return_to URL"** → домен не в `TRUSTED_DOMAINS`
- **Ticket невалиден** → ticket одноразовый
- **Cookie не сохраняется** → используйте `ZID_COOKIE_SECURE=false` для HTTP

## Health check

```bash
curl http://zid-host:5555/health
```