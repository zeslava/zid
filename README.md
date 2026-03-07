# ZID CAS Server

ZID — простой CAS-подобный сервер аутентификации на Rust: пользователь логинится в ZID, ZID выдаёт **one-time ticket**, а ваше приложение подтверждает ticket через `/verify` и получает идентичность пользователя.

## Cookies / SSO (важно для продакшена и локальной разработки)

ZID использует SSO-cookie `zid_sso`, чтобы "узнавать" пользователя и предлагать **Continue** без повторного ввода логина/пароля.

### Secure cookie и локальная разработка

Современные браузеры **не принимают cookie с атрибутом `Secure` по HTTP**. Поэтому:
- в проде ZID должен работать за HTTPS (и cookie должна быть `Secure`)
- локально (если ты запускаешь ZID по `http://localhost:5555`) нужно отключить `Secure`, иначе SSO "не запомнится"

Это управляется переменной окружения `ZID_COOKIE_SECURE`:

- `auto` (по умолчанию): ZID сам определяет HTTPS по заголовкам (`X-Forwarded-Proto=https` / `Forwarded: proto=https`)
- `true` / `1`: всегда ставить `Secure`
- `false` / `0`: никогда не ставить `Secure` (удобно для локальной разработки по HTTP)

Рекомендации:
- **prod**: `ZID_COOKIE_SECURE=auto` (или `true`) + HTTPS + прокси должен прокидывать `X-Forwarded-Proto=https`
- **local http**: `ZID_COOKIE_SECURE=false`

Как это работает на практике:  
1) отправляешь пользователя на ZID (или используешь JSON API),  
2) ZID возвращает ticket (через редирект или прямо в ответе),  
3) твоё приложение обменивает ticket на user info через `/verify`.

---

## Быстрый ориентир (2 минуты)

### В браузере (самый частый сценарий)
1. Ты переводишь пользователя на:
   - `GET http://zid-host:5555/?return_to=https://yourapp.com/callback`
2. Пользователь логинится.
3. ZID делает редирект на:
   - `https://yourapp.com/callback?ticket=<TICKET>`
4. Твой backend вызывает:
   - `POST http://zid-host:5555/verify` с `{ "ticket": "...", "service": "https://yourapp.com/callback" }`
5. Получаешь `{ user_id, username, session_id }`, создаёшь свою сессию/куку.

### Через API (когда не нужен UI/redirect)
1. Клиент вызывает `POST /login` (JSON) **без** `return_to`.
2. ZID возвращает `{ "ticket": "..." }`.
3. Ты вызываешь `/verify` и получаешь пользователя.

---

## Quick Start (Docker)

```bash
docker compose up -d
# ZID будет доступен на http://localhost:5555
```

Проверка здоровья:
```bash
curl -s http://localhost:5555/health
```

---

## Публичные endpoints

| Метод | Endpoint | Для кого | Что делает |
|------:|----------|----------|------------|
| GET   | `/` | Browser | HTML форма логина (поддерживает `return_to`) |
| POST  | `/` | Browser | submit формы логина (`application/x-www-form-urlencoded`) |
| POST  | `/continue` | Browser | подтвердить продолжение как текущий пользователь (SSO) |
| GET   | `/register` | Browser | HTML форма регистрации |
| POST  | `/register` | Browser | submit регистрации (`application/x-www-form-urlencoded`) |
| POST  | `/login` | API | логин JSON → ticket (+опционально redirect_url) |
| POST  | `/login/telegram` | API | Telegram login JSON → ticket (+опционально redirect_url) |
| POST  | `/verify` | Backend | one-time verify ticket → user info |
| POST  | `/logout` | Backend/API | удаление сессии (по `session_id`) |
| GET   | `/health` | Ops | health check |

### OIDC/OAuth 2.0 endpoints (при `OIDC_ENABLED=true`)

| Метод | Endpoint | Описание |
|------:|----------|----------|
| GET   | `/.well-known/openid-configuration` | Discovery (метаданные сервера) |
| GET   | `/oauth/authorize` | Authorization endpoint (code flow) |
| POST  | `/oauth/token` | Token endpoint (обмен code на токены, client_credentials) |
| GET   | `/oauth/userinfo` | UserInfo (Bearer access_token в заголовке `Authorization`) |
| GET   | `/oauth/jwks` | JWKS (публичные ключи для верификации JWT) |

---

## Сценарий: Логин через браузер с редиректом (return_to)

### 1) Отправь пользователя на страницу ZID

Открой в браузере:

`http://localhost:5555/?return_to=https://yourapp.com/callback`

> `return_to` — URL твоего приложения (куда ZID вернёт пользователя).
> Он должен проходить проверку trusted domains (см. `TRUSTED_DOMAINS` ниже).

### 2) Пользователь логинится

Форма отправляется на `POST /` (это делает браузер).

### 3) ZID редиректит обратно

ZID вернёт пользователя на:

`https://yourapp.com/callback?ticket=<uuid>`

Ticket одноразовый и короткоживущий.

### 4) Твоё приложение подтверждает ticket

Запрос:
```bash
curl -X POST http://localhost:5555/verify \
  -H "Content-Type: application/json" \
  -d '{
    "ticket":"<TICKET_FROM_QUERY>",
    "service":"https://yourapp.com/callback"
  }'
```

Ответ (пример):
```json
{
  "success": true,
  "user_id": "176f8257-4bec-4350-99bf-e023186fd04a",
  "username": "alice",
  "session_id": "b2a2d2d6-8c8f-4f65-bfb0-3d2bb4c1c6d9"
}
```

Дальше обычно:
- создаёшь свою сессию (cookie/JWT) и
- редиректишь пользователя в уже защищённый раздел приложения.

---

## Сценарий: Логин через браузер без return_to (без редиректа)

Если `return_to` **не задан**, редиректа **не будет**.

### Пример:

Открой: `http://localhost:5555/`

После логина ZID вернёт HTML-страницу успеха, где будет показан ticket.

Это удобно для ручной отладки и простых интеграций.

---

## Сценарий: Логин через JSON API (без UI)

### Логин без return_to (без редиректа)

Запрос:
```bash
curl -X POST http://localhost:5555/login \
  -H "Content-Type: application/json" \
  -d '{
    "username":"alice",
    "password":"secret123"
  }'
```

Ответ:
```json
{
  "ticket": "4b53f154-3747-463c-9a57-6e856edf4f3a"
}
```

Дальше подтверждаешь ticket через `/verify` так же, как в Flow 1.

### Логин с return_to (если тебе удобен готовый redirect_url)

Запрос:
```bash
curl -X POST http://localhost:5555/login \
  -H "Content-Type: application/json" \
  -d '{
    "username":"alice",
    "password":"secret123",
    "return_to":"https://yourapp.com/callback"
  }'
```

Ответ:
```json
{
  "ticket": "7ee2015f-6112-4db7-adac-d46d03ece91f",
  "redirect_url": "https://yourapp.com/callback?ticket=7ee2015f-6112-4db7-adac-d46d03ece91f"
}
```

> `redirect_url` опционален и возвращается только если `return_to` задан и не пустой.

---

## Сценарий: Регистрация пользователя

### Через браузер
Открой: `http://localhost:5555/register`

### Через curl (form)
```bash
curl -X POST http://localhost:5555/register \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=alice&password=secret123&password_confirm=secret123'
```

---

## Сценарий: Вход через Telegram

Включает вход через Telegram Widget (на странице логина) и endpoint `/login/telegram`.

Нужно настроить env:
```bash
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_BOT_USERNAME=your_bot_username
TELEGRAM_AUTO_REGISTER=true
```

Подробности и примеры: `docs/TELEGRAM_LOGIN.md`

---

## Важные правила (чтобы не удивляться)

### Tickets
- **Одноразовые**: повторный `/verify` для того же ticket должен быть отклонён.
- **TTL ~ 5 минут** (tickеты истекают быстро).
- Ticket привязан к `service` (service URL): при `/verify` нужно передавать тот же сервис, под который ticket выдавался.

### return_to / trusted domains
ZID валидирует `return_to` по списку доверенных доменов.

Настраивается через `TRUSTED_DOMAINS`:
```bash
TRUSTED_DOMAINS=localhost,127.0.0.1,*.local.dev,*.myapp.com,myapp.example.com
```

Для продакшена обязательно добавь домены твоих приложений.

---

## Примеры ошибок (на что смотреть)

### Неверный пароль
- `POST /login` вернёт ошибку (HTTP зависит от обработчика; в HTML — 401 страница Unauthorized).

### Неверный return_to
Если `return_to` не проходит проверку, логин будет отклонён. Проверь `TRUSTED_DOMAINS`.

### Ticket уже использован / просрочен
`/verify` не даст user info.

---

## Конфигурация

### Переменные окружения

```bash
# PostgreSQL
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=zid
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres

# Redis
REDIS_URL=redis://127.0.0.1:6380

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=5555

# return_to allowlist
TRUSTED_DOMAINS=localhost,127.0.0.1,*.local.dev,*.local,*.lan

# Telegram (опционально)
TELEGRAM_BOT_TOKEN=
TELEGRAM_BOT_USERNAME=
TELEGRAM_AUTO_REGISTER=true

# Cookie / SSO
# См. раздел "Cookies / SSO" выше
ZID_COOKIE_SECURE=auto        # auto (по умолчанию) / true / false

# Storage backend (опционально)
SESSION_STORAGE=postgres      # postgres (по умолчанию) или redis
TICKET_STORAGE=postgres       # postgres (по умолчанию) или redis
CREDENTIALS_STORAGE=postgres  # postgres (по умолчанию) или redis

# OIDC/OAuth 2.0 (опционально; по умолчанию включён, при отсутствии конфига/ключей — запуск без OIDC)
# OIDC_ENABLED=true
# OIDC_ISSUER=http://localhost:5555
# OIDC_CLIENTS_FILE=oidc_clients.yaml
# OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem
# OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem
```

### Про storage
- `SESSION_STORAGE` / `TICKET_STORAGE`
  - `postgres` (по умолчанию): хранение в БД
  - `redis`: TTL «из коробки»
- `CREDENTIALS_STORAGE`
  - `postgres` (по умолчанию): credentials в PostgreSQL
  - `redis`: альтернативный вариант

---

## OIDC/OAuth 2.0

ZID поддерживает OIDC/OAuth 2.0 (Authorization Code + PKCE, Client Credentials).

OIDC **включён по умолчанию**. Если файл клиентов или JWT-ключи не настроены, сервер запускается без OIDC (endpoints вернут 503). Чтобы отключить явно: `OIDC_ENABLED=false`.

### Быстрый старт OIDC

1. Сгенерировать RSA-ключи:
   ```bash
   openssl genrsa -out oidc_jwt_private.pem 2048
   openssl rsa -in oidc_jwt_private.pem -pubout -out oidc_jwt_public.pem
   ```

2. Скопировать файл клиентов:
   ```bash
   cp oidc_clients.example.yaml oidc_clients.yaml
   ```

3. Задать переменные окружения:
   ```bash
   OIDC_ENABLED=true
   OIDC_ISSUER=http://localhost:5555
   OIDC_CLIENTS_FILE=oidc_clients.yaml
   OIDC_JWT_PRIVATE_KEY=oidc_jwt_private.pem
   OIDC_JWT_PUBLIC_KEY=oidc_jwt_public.pem
   ```

Подробности и примеры curl: `docs/OIDC_TESTING.md`

---

## Docker: полезное

```bash
# запуск
docker compose up -d

# логи
docker compose logs -f zid-app

# остановка
docker compose down
```

Сервисы по умолчанию:
- App: `http://localhost:5555`
- PostgreSQL: `localhost:5432`
- Redis: `localhost:6380`

---

## Тестирование

E2E тест (регистрация → логин → verify):
```bash
./scripts/test.sh
```

E2E тест OIDC (discovery, client_credentials, jwks):
```bash
./scripts/test-oidc.sh
```

---

## Документация

- `docs/TELEGRAM_LOGIN.md` — интеграция Telegram Login
- `docs/OIDC_TESTING.md` — OIDC/OAuth 2.0: настройка, тесты curl, Authorization Code flow
- `docs/FREEBSD_SETUP.md` — деплой на FreeBSD
