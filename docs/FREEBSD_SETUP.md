# ZID на FreeBSD — Руководство по установке

Это руководство описывает установку и настройку ZID Authentication Service на FreeBSD.

## Требования

- FreeBSD 12.0 или новее
- Rust (для сборки) или готовый бинарник
- PostgreSQL 12+
- Redis 6.0+

## Быстрая установка

### 1. Сборка проекта

```bash
cd /path/to/zid
cargo build --release
```

### 2. Запуск скрипта установки

```bash
sudo sh ./scripts/setup-freebsd.sh ./target/release/zid
```

Скрипт автоматически:
- Создаст пользователя `zid` и группу `zid`
- Установит бинарник в `/usr/local/bin/zid`
- Установит rc.d скрипт в `/usr/local/etc/rc.d/zid`
- Создаст необходимые директории и файлы конфигурации

### 3. Конфигурация

Отредактируйте файл с переменными окружения (по умолчанию `/usr/local/etc/zid/zid.conf`):

```bash
sudo nano /usr/local/etc/zid/zid.conf
```

Убедитесь, что переменные окружения корректны (приложение читает `SERVER_HOST`, `SERVER_PORT`, `POSTGRES_*`, `REDIS_URL` и др. — см. раздел ниже).

### 4. Запуск

**Одноразовый запуск** (команда сразу возвращает управление, сервис работает в фоне):
```bash
sudo service zid start
```

**Автозапуск при загрузке:**
```bash
echo 'zid_enable="YES"' | sudo tee -a /etc/rc.conf
sudo service zid start
```

## Кросс-компиляция с Linux (amd64)

Чтобы собрать бинарник для FreeBSD aarch64 на машине с Linux amd64 (без доступа к FreeBSD):

**Требования:** Docker, Rust (rustup), [cross](https://github.com/cross-rs/cross). Target `aarch64-unknown-freebsd` на хосте (Linux) не поддерживается stable — сборка выполняется внутри Docker-образа cross с nightly и build-std.

```bash
# Один раз: установить cross
cargo install cross

# Сборка (внутри контейнера используется nightly для build-std)
task cross-freebsd-aarch64
```

Артефакт: `./target/aarch64-unknown-freebsd/release/zid`. Скопируйте его на FreeBSD (например через `scp`) и установите по текущему сценарию:

```bash
# На FreeBSD после копирования бинарника
sudo sh ./scripts/setup-freebsd.sh ./target/aarch64-unknown-freebsd/release/zid
```

В корне проекта задан `Cross.toml` с образом для target `aarch64-unknown-freebsd`. Если образ `ghcr.io/cross-rs/aarch64-unknown-freebsd:latest` не найден при сборке, соберите его из репозитория cross-rs:

```bash
git clone --depth 1 https://github.com/cross-rs/cross && cd cross/docker && docker build -f Dockerfile.aarch64-unknown-freebsd -t ghcr.io/cross-rs/aarch64-unknown-freebsd:latest .
```

Если Docker недоступен, потребуется ручная настройка: кросс-линкер и sysroot FreeBSD aarch64, указание линкера в `.cargo/config.toml` для target `aarch64-unknown-freebsd`.

## Команды управления

| Команда | Описание |
|---------|----------|
| `sudo service zid start` | Запуск сервиса |
| `sudo service zid stop` | Остановка сервиса |
| `sudo service zid restart` | Перезагрузка сервиса |
| `sudo service zid status` | Проверка статуса |
| `sudo service zid config` | Показать конфигурацию |
| `sudo service zid logs` | Просмотр логов (tail -f) |

## Структура файлов

```
/usr/local/bin/zid                 # Исполняемый файл сервиса
/usr/local/etc/rc.d/zid            # RC.D скрипт (управление сервисом)
/usr/local/etc/zid/                # Каталог конфигурации ZID
  zid.conf                         # Файл переменных окружения (zid_env_file)
  oidc_clients.yaml                 # OIDC: клиенты (если включён OIDC)
  oidc_jwt_private.pem             # OIDC: ключ подписи JWT
  oidc_jwt_public.pem              # OIDC: публичный ключ (JWKS)
/var/lib/zid/                      # Домашняя директория пользователя zid
/var/log/zid/zid.log               # Логи сервиса
/var/run/zid/zid.pid               # PID файл
```

## Конфигурирование rc.conf

Добавьте в `/etc/rc.conf` (для автозапуска):

```bash
# Основные параметры
zid_enable="YES"

# Опциональные параметры (значения по умолчанию)
zid_user="zid"                           # Unix-пользователь
zid_group="zid"                          # Unix-группа
zid_env_file="/usr/local/etc/zid/zid.conf"   # Файл переменных окружения (см. rc.subr(8))
zid_logfile="/var/log/zid/zid.log"       # Файл логов
zid_pidfile="/var/run/zid/zid.pid"       # PID файл
```

Переменная `zid_env_file` — стандартная для rc.subr(8): rc.d автоматически подхватывает из неё переменные окружения при старте. Для совместимости поддерживается устаревший синоним `zid_config`.

## Переменные окружения (в файле zid_env_file)

```bash
# Адрес и порт (приложение читает SERVER_HOST, SERVER_PORT)
SERVER_HOST="0.0.0.0"
SERVER_PORT="5555"

# Хранилища (postgres по умолчанию, redis как альтернатива)
SESSION_STORAGE="postgres"
TICKET_STORAGE="postgres"
CREDENTIALS_STORAGE="postgres"

# База данных PostgreSQL
DATABASE_URL="postgresql://user:pass@localhost/zid"

# Redis
REDIS_URL="redis://localhost:6379"

# Telegram (опционально)
TELEGRAM_BOT_USERNAME="your_bot"
TELEGRAM_BOT_TOKEN="your_token"

# Безопасность
ZID_COOKIE_SECURE="false"           # true для HTTPS, false для локальной разработки

# Логирование
RUST_LOG="info"                     # Уровни: trace, debug, info, warn, error
RUST_BACKTRACE="1"                  # Backtrace при панике

# Доверенные домены (для return_to редиректов)
TRUSTED_DOMAINS="localhost:3000,app.example.com,api.example.com"
```

## Зависимости

### PostgreSQL

Убедитесь, что PostgreSQL работает и доступна база данных:

```bash
# Проверка статуса
sudo service postgresql status

# Создание БД и пользователя (если не существуют)
sudo -u postgres createuser -P zid
sudo -u postgres createdb -O zid zid

# Применение миграций
cd /path/to/zid
sqlx migrate run --database-url="postgresql://zid:pass@localhost/zid"
```

### Redis

Убедитесь, что Redis работает:

```bash
# Проверка статуса
sudo service redis status

# Запуск если отключен
sudo service redis start

# Добавить в /etc/rc.conf для автозапуска
echo 'redis_enable="YES"' | sudo tee -a /etc/rc.conf
```

## Интеграция с Nginx (реверс-прокси)

Пример конфига для Nginx:

```nginx
upstream zid {
    server 127.0.0.1:3000;
}

server {
    listen 80;
    server_name auth.example.com;

    location / {
        proxy_pass http://zid;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Добавьте конфиг в `/usr/local/etc/nginx/conf.d/`:

```bash
sudo cp zid-nginx.conf /usr/local/etc/nginx/conf.d/
sudo service nginx reload
```

## Мониторинг и логирование

### Просмотр логов

```bash
# Реальное время
sudo tail -f /var/log/zid/zid.log

# Или через rc.d команду
sudo service zid logs
```

### Ротация логов

Добавьте в `/etc/newsyslog.conf`:

```
/var/log/zid/zid.log   zid:zid   644  7  *  @daily  Z
```

### Мониторинг здоровья

Проверьте health-check endpoint:

```bash
curl http://localhost:3000/health
```

## Троубл-шутинг

### Сервис не стартует

1. Проверьте логи:
   ```bash
   sudo tail -50 /var/log/zid/zid.log
   ```

2. Проверьте конфигурацию:
   ```bash
   sudo service zid config
   ```

3. Убедитесь, что зависимости запущены:
   ```bash
   sudo service postgresql status
   sudo service redis status
   ```

### Ошибка подключения к БД

```bash
# Проверьте DATABASE_URL в файле zid_env_file (по умолчанию /usr/local/etc/zid/zid.conf)
sudo cat /usr/local/etc/zid/zid.conf | grep DATABASE_URL

# Тестируйте подключение
psql "postgresql://zid:pass@localhost/zid" -c "SELECT 1"
```

### Ошибка прав доступа

Проверьте права на файлы:

```bash
ls -la /var/log/zid/
ls -la /var/run/zid/
ls -la /usr/local/etc/zid/zid.conf
```

Исправьте если нужно:

```bash
sudo chown zid:zid /var/log/zid/
sudo chown zid:zid /var/run/zid/
```

### Проверка портов

```bash
# Проверьте, что порт 3000 слушается
sudo sockstat -l | grep 3000

# Если есть конфликт, измените SERVER_PORT в файле zid_env_file
```

## Обновление

### Обновление бинарника

```bash
# 1. Соберите новую версию
cd /path/to/zid
git pull
cargo build --release

# 2. Переустановите с новым бинарником
sudo sh ./scripts/setup-freebsd.sh ./target/release/zid

# 3. Перезагрузите сервис
sudo service zid restart

# 4. Проверьте статус
sudo service zid status
```

### Откат

```bash
sudo service zid stop
# Восстановите старый бинарник вручную
sudo service zid start
```

## Удаление

```bash
# 1. Остановите сервис
sudo service zid stop

# 2. Удалите из автозапуска
sudo sed -i '' '/zid_enable/d' /etc/rc.conf

# 3. Удалите файлы
sudo rm /usr/local/bin/zid
sudo rm /usr/local/etc/rc.d/zid
sudo rm /usr/local/etc/zid/zid.conf

# 4. Удалите пользователя и директории (опционально)
sudo pw userdel zid
sudo rm -rf /var/lib/zid /var/log/zid /var/run/zid
```

## Дополнительно

Смотрите также:
- [ZID Integration Guide](../ZID_INTEGRATION_GUIDE.md)
- [Telegram Login](../docs/TELEGRAM_LOGIN.md)
- [AGENTS.md](../AGENTS.md) — Архитектура проекта
