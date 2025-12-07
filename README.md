# ZID CAS Server

Простой и эффективный CAS (Central Authentication Service) сервер на Rust.

## 🚀 Quick Start

```bash
# Запуск
docker compose up -d

# Или через Makefile
make start
```

Сервер будет доступен на http://localhost:5555

## ✨ Особенности

- ✅ **Argon2id** хеширование паролей
- ✅ **One-time use** тикеты с TTL
- ✅ **Service URL** валидация
- ✅ PostgreSQL + Redis
- ✅ Docker Compose

## 📋 API

| Метод | Endpoint | Описание |
|-------|----------|----------|
| GET | `/` | HTML форма логина |
| POST | `/` | Отправка формы логина (form data) |
| POST | `/login` | Аутентификация через JSON API |
| POST | `/register` | Регистрация нового пользователя |
| POST | `/verify` | Верификация тикета (one-time use) |
| POST | `/logout` | Удаление сессии |
| GET | `/health` | Health check |

### Регистрация
```bash
curl -X POST http://localhost:5555/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","password":"secret123"}'
```

### Логин (JSON API)
```bash
curl -X POST http://localhost:5555/login \
  -H "Content-Type: application/json" \
  -d '{
    "username":"alice",
    "password":"secret123",
    "return_to":"http://localhost:3000"
  }'
```

### Логин (HTML форма)
Откройте в браузере: http://localhost:5555/

**Ответ:**
```json
{
  "ticket": "abc-123...",
  "redirect_url": "http://localhost:3000?ticket=abc-123..."
}
```

### Верификация тикета
```bash
curl -X POST http://localhost:5555/verify \
  -H "Content-Type: application/json" \
  -d '{
    "ticket":"abc-123...",
    "service":"http://localhost:3000"
  }'
```

**Ответ:**
```json
{
  "success": true,
  "user_id": "...",
  "username": "alice",
  "session_id": "..."
}
```

⚠️ **Важно:** Тикеты одноразовые и удаляются после верификации!

### Logout
```bash
curl -X POST http://localhost:5555/logout \
  -H "Content-Type: application/json" \
  -d '{"session_id":"..."}'
```

## 🔐 Безопасность

### Argon2id Password Hashing
- Memory-hard (19 MB, 2 iterations)
- Уникальная соль на пароль
- OWASP recommended
- Constant-time сравнение

**Формат хеша:**
```
$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
```

### Ticket Security
- ✅ One-time use
- ✅ 5-минутный TTL
- ✅ Привязка к service URL
- ✅ Trusted domains only

**Подробнее:** [docs/SECURITY.md](docs/SECURITY.md)

## 🐳 Docker

```bash
# Запуск
docker compose up -d

# Логи
make logs

# Остановка
make down
```

**Сервисы:**
- App: http://localhost:5555
- PostgreSQL: localhost:5432
- Redis: localhost:6380

## 🧪 Тестирование

```bash
# End-to-end тест (регистрация → логин → верификация)
./scripts/test.sh
```

**Тест проверяет:**
- ✅ Регистрацию пользователей
- ✅ Логин через JSON API
- ✅ Верификацию тикета
- ✅ One-time use (повторная верификация отклоняется)
- ✅ Проверку service URL

## 📦 Зависимости

```toml
[dependencies]
anyhow = "1.0"
argon2 = "0.5"
axum = "0.8"
postgres = "0.19"
r2d2 = "0.8"
r2d2_postgres = "0.18"
rand_core = { version = "0.6", features = ["getrandom"] }
redis = "0.27"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.48", features = ["full"] }
url = "2.5"
uuid = { version = "1.19", features = ["v4"] }
```

## 🗄️ База данных

### PostgreSQL
```sql
CREATE TABLE users (
    id VARCHAR(36) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Redis Keys
```
credentials:username:{username} → Argon2 hash
session:id:{session_id} → Session data
ticket:id:{ticket_id} → Ticket data (TTL: 5m)
```

## 📚 Документация

- [API Reference](docs/API_REFERENCE.md) - Полный API
- [Security](docs/SECURITY.md) - Безопасность
- [Ticket Verification](docs/TICKET_VERIFICATION.md) - Детали верификации
- [Docker Guide](DOCKER.md) - Docker deployment
- [Quick Start](QUICKSTART.md) - Быстрый старт

## 🛠️ Разработка

### Локальный запуск (без Docker)

```bash
# PostgreSQL
createdb zid

# Запуск
cargo run --release
```

### Makefile команды
```bash
make help         # Все команды
make build        # Собрать
make logs         # Логи
make db-shell     # PostgreSQL shell
make redis-cli    # Redis CLI
```

## ✅ TODO

- [x] Argon2 password hashing
- [x] Ticket verification
- [x] User registration
- [x] Health check
- [ ] Logging (tracing)
- [ ] Rate limiting
- [ ] CSRF protection
- [ ] Password requirements
- [ ] Metrics (Prometheus)

## 📝 Лицензия

MIT

---

**Полная документация:** [docs/](docs/)