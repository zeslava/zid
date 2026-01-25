# AGENTS.md — Правила для AI-агентов

Этот документ содержит инструкции для AI-агентов по разработке проекта ZID.

## Обзор проекта

**ZID** — CAS-подобный сервер аутентификации на Rust.

Принцип работы:
1. Пользователь логинится в ZID
2. ZID выдаёт **one-time ticket**
3. Приложение верифицирует ticket через `/verify` → получает `user_id`, `username`
4. Приложение создаёт свою сессию

### Основные сущности

- **User** — пользователь (поддержка username/password и Telegram)
- **Session** — SSO-сессия (7 дней, sliding expiration)
- **Ticket** — одноразовый тикет (5 минут TTL)
- **Credentials** — учётные данные (Argon2 хеширование)

## Архитектура

Проект использует **Hexagonal Architecture** (Ports & Adapters):

```
src/
├── ports/           # Доменные интерфейсы (traits)
│   ├── entities.rs      # Сущности: User, Session, Ticket
│   ├── error.rs         # Типы ошибок домена
│   ├── zid_service.rs   # Главный сервис (trait)
│   └── *_repository.rs  # Интерфейсы репозиториев
├── adapters/        # Реализации инфраструктуры
│   ├── http/            # Axum: handlers, routes, DTOs
│   ├── persistence/     # PostgreSQL и Redis реализации
│   └── telegram/        # Telegram Login Widget
├── application/     # Бизнес-логика
│   └── zid_app.rs       # Реализация ZidService
└── main.rs          # Точка входа, DI
```

### Ключевые файлы

| Файл | Описание |
|------|----------|
| `src/ports/entities.rs` | Доменные сущности |
| `src/ports/error.rs` | Типы ошибок |
| `src/application/zid_app.rs` | Бизнес-логика аутентификации |
| `src/adapters/http/handlers.rs` | HTTP обработчики |
| `src/adapters/http/routes.rs` | Маршруты API |
| `src/adapters/http/dto.rs` | DTO для HTTP |

## Правила разработки

### Архитектурные правила

1. **Направление зависимостей**: Adapters зависят от Ports, не наоборот
   ```rust
   // adapters/persistence/postgres_user.rs
   use crate::ports::{entities::User, error::Error, user_repository::UserRepository};
   ```

2. **Новые репозитории**:
   - Trait определяется в `src/ports/`
   - Реализации в `src/adapters/persistence/` (postgres_*, redis_*)

3. **Async/Sync граница**: HTTP handlers (async) вызывают синхронный доменный код через `spawn_blocking`:
   ```rust
   let result = tokio::task::spawn_blocking(move || {
       state.zid.login(&req.username, &req.password, req.return_to.as_deref())
   })
   .await??;
   ```

4. **Dependency Injection**: Репозитории инжектируются в `ZidApp` через `Arc<dyn Trait>`

### Конвенции кода

1. **Язык документации**: Комментарии и документация на **русском языке**

2. **Форматирование строк**: Переменные можно использовать напрямую в `format!`:
   ```rust
   // Хорошо
   format!("User {username} not found")
   // Избегать
   format!("User {} not found", username)
   ```

3. **Именование**:
   - Репозитории: `PostgresXxxRepository`, `RedisXxxRepository`
   - Traits: `XxxRepository`, `XxxService`
   - DTOs: `XxxRequest`, `XxxResponse`
   - Ошибки: PascalCase (`UserNotFound`, `TicketExpired`)

4. **Обработка ошибок**:
   - Доменные ошибки через `ports::error::Error`
   - Маппинг DB ошибок в доменные в репозиториях
   - HTTP ошибки через `HttpError` wrapper

5. **Конструкторы**: Использовать `new()` методы:
   ```rust
   impl PostgresUserRepository {
       pub fn new(pool: Pool<...>) -> Self {
           PostgresUserRepository { pool }
       }
   }
   ```

6. **UUID**: Генерация через `uuid::Uuid::new_v4().to_string()`

### Тестирование

1. **Тесты с внешними зависимостями**: Помечать `#[ignore]`
   ```rust
   #[test]
   #[ignore] // Требует запущенный PostgreSQL
   fn test_user_repository() { ... }
   ```

2. **Helper функции**: `setup_test_*()` для инфраструктуры тестов

3. **Расположение**: `#[cfg(test)] mod tests` в конце файла реализации

## Команды разработки

```bash
# Docker
task up          # Запуск Docker сервисов
task down        # Остановка Docker сервисов

# Сборка и запуск
task build       # Сборка Rust приложения
task run         # Запуск приложения локально

# База данных
task migrate     # Запуск миграций (sqlx-cli)

# Тестирование
./scripts/test.sh # E2E тесты
cargo test        # Unit тесты
```

Дополнительно через docker compose:

```bash
docker compose logs -f zid-app  # Логи приложения
docker compose ps               # Статус сервисов
```

## Переменные окружения

| Переменная | Значения | Описание |
|------------|----------|----------|
| `SESSION_STORAGE` | `redis` (default), `postgres` | Хранилище сессий |
| `TICKET_STORAGE` | `redis` (default), `postgres` | Хранилище тикетов |
| `CREDENTIALS_STORAGE` | `postgres` (default), `redis` | Хранилище credentials |
| `TRUSTED_DOMAINS` | comma-separated | Доверенные домены для return_to |
| `ZID_COOKIE_SECURE` | `auto`, `true`, `false` | Secure флаг для cookie |

Полный список в `.env.example`.

## Запреты

1. **Не использовать `sudo`** — команды требующие sudo отдавать пользователю
2. **Не создавать тестовые примеры** без явного запроса
3. **Не менять архитектуру** без обсуждения (Ports & Adapters)

## HTTP API

| Метод | Endpoint | Описание |
|-------|----------|----------|
| GET | `/` | HTML форма логина |
| POST | `/` | Submit формы логина |
| GET | `/register` | HTML форма регистрации |
| POST | `/register` | Submit регистрации |
| POST | `/login` | JSON API логин |
| POST | `/login/telegram` | Telegram логин |
| POST | `/verify` | Верификация тикета |
| POST | `/logout` | Удаление сессии |
| GET | `/health` | Health check |

## Миграции

Миграции находятся в `migrations/` и применяются через **sqlx-cli** (команда `task migrate`).

### Формат миграций

Миграции используют формат с суффиксами `.up.sql` и `.down.sql` для поддержки откатов:

```
migrations/
├── 001_create_users_table.up.sql
├── 001_create_users_table.down.sql
├── 002_add_telegram_support.up.sql
├── 002_add_telegram_support.down.sql
└── ...
```

### Создание новых миграций

```bash
# Создать реверсивную миграцию (с .up.sql и .down.sql)
task migrate-add NAME=description

# Или напрямую через sqlx-cli с флагом -r
sqlx migrate add -r description
```

### Применение и откат

```bash
# Применить все миграции
task migrate

# Откатить последнюю миграцию
task migrate-revert
```

**Важно**: sqlx-cli выполняет `.up.sql` при применении и `.down.sql` при откате.

### Паттерны в миграциях

```sql
-- Использовать IF NOT EXISTS / IF EXISTS для идемпотентности
CREATE TABLE IF NOT EXISTS ...
ALTER TABLE ... ADD COLUMN IF NOT EXISTS ...
DROP TABLE IF EXISTS ...

-- Логирование завершения миграции
DO $$
BEGIN
    RAISE NOTICE 'Migration completed successfully';
END $$;
```

### Docker и миграции

При первом запуске PostgreSQL через Docker миграции применяются автоматически из `/docker-entrypoint-initdb.d`, но для управления миграциями в продакшене используйте `task migrate`.

## Зависимости

Основные:
- **axum** (0.8) — веб-фреймворк
- **tokio** (1.48) — async runtime
- **postgres** + **r2d2** — PostgreSQL
- **redis** — Redis клиент
- **argon2** — хеширование паролей
- **serde** / **serde_json** — сериализация
