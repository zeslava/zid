# Telegram Login Integration

Полная инструкция по интеграции входа через Telegram в ZID CAS Server.

## 📋 Содержание

- [Быстрый старт](#быстрый-старт)
- [Подробная настройка](#подробная-настройка)
- [API](#api)
- [Безопасность](#безопасность)
- [Локальная разработка](#локальная-разработка)
- [Troubleshooting](#troubleshooting)

## 🚀 Быстрый старт

### 1. Создайте Telegram бота

1. Откройте Telegram и найдите [@BotFather](https://t.me/botfather)
2. Отправьте команду `/newbot`
3. Следуйте инструкциям:
   - Введите имя бота (например: "My CAS Bot")
   - Введите username бота (например: "my_cas_bot", должен заканчиваться на "_bot")
4. Сохраните **токен бота** (выглядит так: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)

### 2. Настройте домен для Login Widget

⚠️ **Важно:** Telegram Login Widget работает только с публичными доменами или localhost.

Отправьте команду `/setdomain` в @BotFather:
```
/setdomain
→ Выберите вашего бота
→ Введите домен: localhost (для разработки) или ваш реальный домен
```

Для продакшена укажите ваш реальный домен без протокола:
- ✅ `cas.example.com`
- ✅ `localhost` (только для разработки)
- ❌ `https://cas.example.com` (неправильно - не указывайте протокол)

### 3. Настройте переменные окружения

Создайте файл `.env` в корне проекта:

```bash
# Токен бота из @BotFather
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz

# Username бота БЕЗ символа @ (например: my_cas_bot)
TELEGRAM_BOT_USERNAME=my_cas_bot

# Автоматически создавать новых пользователей при первом входе через Telegram
# true - любой пользователь Telegram может войти
# false - только существующие пользователи могут войти через Telegram
TELEGRAM_AUTO_REGISTER=true
```

### 4. Запустите сервер

```bash
# С Docker Compose (рекомендуется)
docker compose up -d

# Или локально
cargo run --release
```

### 5. Проверьте работу

Откройте в браузере: http://localhost:5555/

Вы должны увидеть:
- Обычную форму входа (логин/пароль)
- Разделитель "OR"
- Кнопку "Login with Telegram"

## 📝 Подробная настройка

### Переменные окружения

| Переменная | Обязательна | Описание | Пример |
|------------|-------------|----------|--------|
| `TELEGRAM_BOT_TOKEN` | Да* | Токен бота из @BotFather | `123456789:ABC...` |
| `TELEGRAM_BOT_USERNAME` | Да* | Username бота без @ | `my_cas_bot` |
| `TELEGRAM_AUTO_REGISTER` | Нет | Автоматическая регистрация | `true` (по умолчанию) |

*\* Если хотя бы одна из переменных не установлена, кнопка Telegram Login не будет показана*

### Режимы работы

#### Режим 1: Автоматическая регистрация (по умолчанию)

```bash
TELEGRAM_AUTO_REGISTER=true
```

**Поведение:**
- Любой пользователь Telegram может войти
- При первом входе автоматически создается аккаунт
- Username в БД: `tg_{telegram_username}` или `tg_{telegram_id}`

**Подходит для:**
- Публичных сервисов
- Когда нужен простой вход без предварительной регистрации

#### Режим 2: Только существующие пользователи

```bash
TELEGRAM_AUTO_REGISTER=false
```

**Поведение:**
- Только пользователи с привязанным Telegram ID могут войти
- При попытке входа неизвестного пользователя возвращается ошибка
- Администратор должен вручную создать пользователей

**Подходит для:**
- Корпоративных систем
- Когда нужен контроль над списком пользователей

### Миграция базы данных

При запуске Docker Compose миграция применяется автоматически.

Для ручного применения:

```bash
# Подключитесь к PostgreSQL
psql -U postgres -d zid -h localhost -p 5432

# Примените миграцию
\i migrations/002_add_telegram_support.sql
```

Миграция добавляет следующие поля в таблицу `users`:
- `telegram_id` (BIGINT, UNIQUE) - ID пользователя в Telegram
- `telegram_username` (VARCHAR) - @username в Telegram
- `telegram_first_name` (VARCHAR) - Имя
- `telegram_last_name` (VARCHAR) - Фамилия
- `telegram_photo_url` (TEXT) - URL аватарки
- `telegram_auth_date` (BIGINT) - Время последней аутентификации

## 📡 API

### POST /login/telegram

Endpoint для аутентификации через Telegram. Вызывается автоматически JavaScript кодом после успешного входа через Telegram Widget.

**Request:**
```json
{
  "id": 123456789,
  "first_name": "John",
  "last_name": "Doe",
  "username": "johndoe",
  "photo_url": "https://t.me/i/userpic/320/...",
  "auth_date": 1234567890,
  "hash": "abc123def456...",
  "return_to": "http://localhost:3000"
}
```

**Response (успех):**
```json
{
  "ticket": "ZID-7a3b9c2f8e1d4a5b6c7d8e9f0a1b2c3d",
  "redirect_url": "http://localhost:3000?ticket=ZID-7a3b9c2f8e1d4a5b6c7d8e9f0a1b2c3d"
}
```

**Response (ошибка):**
```json
{
  "error": "Telegram auth verification failed: Hash mismatch"
}
```

### Коды ошибок

| Код | Описание | Причина |
|-----|----------|---------|
| 401 | Unauthorized | Hash verification failed - подпись Telegram не прошла проверку |
| 404 | Not Found | User not found - пользователь не найден (когда `TELEGRAM_AUTO_REGISTER=false`) |
| 500 | Internal Server Error | TELEGRAM_BOT_TOKEN не настроен или ошибка БД |

## 🔐 Безопасность

### Проверка подлинности данных

Все данные от Telegram проверяются на подлинность согласно [официальной документации](https://core.telegram.org/widgets/login#checking-authorization):

1. **Создание data_check_string:**
   - Все поля (кроме `hash`) собираются в формате `key=value`
   - Сортируются по алфавиту
   - Объединяются через `\n`

2. **Вычисление secret_key:**
   ```
   secret_key = SHA256(bot_token)
   ```

3. **Вычисление hash:**
   ```
   hash = HMAC-SHA256(data_check_string, secret_key)
   ```

4. **Сравнение:**
   - Полученный hash сравнивается с hash от Telegram
   - Используется constant-time сравнение для защиты от timing attacks

5. **Проверка времени:**
   - `auth_date` не должен быть старше 24 часов
   - Защита от replay attacks

### Что это дает?

✅ Невозможно подделать данные от имени другого пользователя  
✅ Невозможно повторно использовать старые данные авторизации  
✅ Только реальные пользователи Telegram могут войти  
✅ Токен бота известен только серверу (не передается клиенту)

### Рекомендации по безопасности

1. **Храните токен бота в секрете:**
   - Используйте `.env` файл (добавьте в `.gitignore`)
   - Никогда не коммитьте токен в Git
   - В продакшене используйте secrets management (Docker secrets, K8s secrets, etc.)

2. **Используйте HTTPS в продакшене:**
   - Telegram требует HTTPS для Login Widget (кроме localhost)
   - Получите SSL сертификат (Let's Encrypt бесплатный)

3. **Настройте TRUSTED_DOMAINS:**
   ```bash
   TRUSTED_DOMAINS=cas.example.com,app.example.com
   ```

4. **Регулярно обновляйте зависимости:**
   ```bash
   cargo update
   ```

## 💻 Локальная разработка

### Проблема: Telegram требует публичный домен

Telegram Login Widget **не работает** с IP адресами и внутренними доменами (кроме `localhost`).

### Решение 1: ngrok (рекомендуется для разработки)

```bash
# Установите ngrok: https://ngrok.com/download

# Запустите туннель
ngrok http 5555

# Скопируйте публичный URL (например: https://abc123.ngrok.io)
# Настройте в @BotFather через /setdomain
```

**Преимущества:**
- ✅ Быстро и просто
- ✅ Автоматический HTTPS
- ✅ Работает из любой сети

**Недостатки:**
- ❌ URL меняется при каждом перезапуске (в бесплатной версии)
- ❌ Требует обновления домена в @BotFather

### Решение 2: localhost (только для начальной разработки)

```bash
# В @BotFather: /setdomain → localhost
```

**Преимущества:**
- ✅ Не нужен внешний сервис
- ✅ Работает оффлайн

**Недостатки:**
- ❌ Работает только локально (нельзя тестировать с мобильного)
- ❌ В продакшене нужно будет менять домен

### Решение 3: Tailscale (для команд)

```bash
# Установите Tailscale: https://tailscale.com/download

# Включите MagicDNS и HTTPS сертификаты
# Ваша машина будет доступна как: your-machine.tail-xxxxx.ts.net

# В @BotFather: /setdomain → your-machine.tail-xxxxx.ts.net
```

**Преимущества:**
- ✅ Постоянный домен
- ✅ Автоматический HTTPS
- ✅ Доступ для всей команды
- ✅ Работает в любой сети

**Недостатки:**
- ❌ Требует установки Tailscale

## 🐛 Troubleshooting

### Кнопка Telegram не появляется

**Причина:** Не настроены переменные окружения.

**Решение:**
1. Проверьте, что установлены `TELEGRAM_BOT_TOKEN` и `TELEGRAM_BOT_USERNAME`
2. Проверьте логи:
   ```bash
   docker compose logs zid-app
   ```
3. Перезапустите контейнер:
   ```bash
   docker compose restart zid-app
   ```

### "Bot domain invalid"

**Причина:** Домен не настроен в @BotFather.

**Решение:**
1. Отправьте `/setdomain` в @BotFather
2. Выберите вашего бота
3. Укажите домен (без `https://`):
   - Для разработки: `localhost`
   - Для ngrok: `abc123.ngrok.io`
   - Для продакшена: `cas.example.com`

### "Hash verification failed"

**Причина 1:** Неправильный токен бота.

**Решение:**
- Проверьте, что `TELEGRAM_BOT_TOKEN` правильный
- Скопируйте токен из @BotFather заново

**Причина 2:** Рассинхронизация времени на сервере.

**Решение:**
```bash
# Проверьте время на сервере
date

# Синхронизируйте время (Linux)
sudo ntpdate -s time.nist.gov
```

### "Auth data is too old"

**Причина:** Данные старше 24 часов (или время на сервере неправильное).

**Решение:**
1. Проверьте время на сервере
2. Попробуйте войти заново (не используйте кешированные данные)

### "User not found" (при TELEGRAM_AUTO_REGISTER=false)

**Причина:** Пользователь не существует в БД.

**Решение:**

**Вариант 1:** Включите автоматическую регистрацию:
```bash
TELEGRAM_AUTO_REGISTER=true
```

**Вариант 2:** Создайте пользователя вручную:
```sql
-- Подключитесь к БД
psql -U postgres -d zid -h localhost -p 5432

-- Создайте пользователя
INSERT INTO users (id, username, telegram_id, telegram_username, telegram_first_name)
VALUES (
    gen_random_uuid()::text,
    'johndoe',
    123456789,  -- ID пользователя из Telegram
    'johndoe',  -- @username
    'John'
);
```

### CORS ошибки в браузере

**Причина:** Telegram Widget загружается с другого домена.

**Решение:** Это нормально. CORS ошибки от Telegram Widget можно игнорировать - они не влияют на работу.

### Не работает на мобильном устройстве

**Причина:** `localhost` недоступен с мобильного устройства.

**Решение:** Используйте ngrok или Tailscale (см. раздел "Локальная разработка").

## 📊 Примеры использования

### Пример 1: Публичный сервис с автоматической регистрацией

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=my_cas_bot
TELEGRAM_AUTO_REGISTER=true
TRUSTED_DOMAINS=cas.example.com,app1.example.com,app2.example.com
```

**Результат:**
- Любой пользователь Telegram может войти
- При первом входе создается аккаунт
- Тикет можно использовать для всех доменов в `TRUSTED_DOMAINS`

### Пример 2: Корпоративная система с контролем доступа

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=company_auth_bot
TELEGRAM_AUTO_REGISTER=false
TRUSTED_DOMAINS=*.company.com
```

**Результат:**
- Только пользователи с привязанным Telegram ID могут войти
- HR или админ создает пользователей в БД
- Поддерживаются все поддомены company.com

### Пример 3: Гибридная аутентификация

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=my_bot
TELEGRAM_AUTO_REGISTER=true
```

**Пользователи могут:**
1. Зарегистрироваться через форму (логин + пароль)
2. Войти через Telegram (создается новый аккаунт)
3. Привязать Telegram к существующему аккаунту (TODO: feature)

## 🔗 Полезные ссылки

- [Официальная документация Telegram Login Widget](https://core.telegram.org/widgets/login)
- [Проверка авторизации](https://core.telegram.org/widgets/login#checking-authorization)
- [@BotFather](https://t.me/botfather) - создание и настройка ботов
- [ngrok](https://ngrok.com/) - туннелинг для локальной разработки
- [Tailscale](https://tailscale.com/) - VPN для команд

## 📞 Поддержка

Если у вас возникли проблемы:

1. Проверьте раздел [Troubleshooting](#troubleshooting)
2. Посмотрите логи: `docker compose logs zid-app`
3. Создайте issue на GitHub с описанием проблемы

---

**Версия документа:** 1.0  
**Дата обновления:** 2024