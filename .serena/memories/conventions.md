# Конвенции кода

- **Комментарии и доки — на русском** (конвенция проекта), идентификаторы и логи — на английском.
- `format!("User {username}")`, а не `format!("User {}", username)`.
- Именование: `PostgresXxxRepository` / `RedisXxxRepository` / `SqliteXxxRepository`; трейты `XxxRepository`, `XxxService`; DTO `XxxRequest` / `XxxResponse`; варианты ошибок PascalCase (`UserNotFound`, `TicketExpired`).
- Конструкторы — `pub fn new(...) -> Self`.
- UUID: `uuid::Uuid::new_v4().to_string()` (id хранятся строками).
- Ошибки: доменные — `ports::error::Error`; ошибки БД мапятся в доменные внутри репозитория; в HTTP-слое — обёртка `HttpError` → статус-код.
- Тесты: `#[cfg(test)] mod tests` в конце файла реализации; всё, что требует PostgreSQL/Redis, помечать `#[ignore]`; хелперы `setup_test_*()`.
- HTML-страницы (логин, регистрация, ошибки) формируются строками прямо в `handlers.rs` — шаблонизатора нет, при правках держать единый стиль соседних функций; статика (favicon и т.п.) — `static/`, вшивается через rust-embed.
- Формы защищены CSRF-токеном; POST-хендлеры проверяют его до всякой логики.
- Новая сущность хранилища = трейт в `src/ports/` + реализации на все нужные бэкенды в `src/adapters/persistence/` + ветка выбора в `main.rs`.
- Миграции идемпотентны (`IF NOT EXISTS` / `IF EXISTS`), парой `.up.sql`/`.down.sql`; новый файл нужно добавить в массив `MIGRATIONS` в `src/migrations.rs`, иначе он не применится на старте.
- SQL — только параметризованные запросы.
- Не добавлять примеры/тесты/логирование без явного запроса.
