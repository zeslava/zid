#!/bin/sh
# Скрипт установки ZID сервиса на FreeBSD
# Использование: sudo ./setup-freebsd.sh [путь_к_бинарнику]
# По умолчанию: ./target/release/zid

set -e

# Конфигурация
ZID_USER="zid"
ZID_GROUP="zid"
ZID_HOME="/var/lib/zid"
ZID_LOG_DIR="/var/log/zid"
ZID_RUN_DIR="/var/run/zid"
ZID_BIN_PATH="${1:-./target/release/zid}"

echo "=== ZID FreeBSD Setup ==="
echo ""

# Проверка прав администратора
if [ "$(id -u)" != "0" ]; then
    echo "Ошибка: скрипт должен быть запущен с правами root (используйте sudo)"
    exit 1
fi

# Проверка наличия бинарника
if [ ! -f "$ZID_BIN_PATH" ]; then
    echo "Ошибка: бинарник не найден: $ZID_BIN_PATH"
    echo "Сначала соберите проект: cargo build --release"
    exit 1
fi

echo "📦 Установка бинарника..."
install -m 0755 "$ZID_BIN_PATH" /usr/local/bin/zid
echo "✓ Бинарник установлен"
echo ""

# Создание пользователя и группы
echo "👤 Создание пользователя и группы..."

if pw groupshow "$ZID_GROUP" > /dev/null 2>&1; then
    echo "  Группа $ZID_GROUP уже существует"
else
    pw groupadd -n "$ZID_GROUP"
    echo "  Создана группа $ZID_GROUP"
fi

if pw usershow "$ZID_USER" > /dev/null 2>&1; then
    echo "  Пользователь $ZID_USER уже существует"
else
    pw useradd "$ZID_USER" \
        -d "$ZID_HOME" \
        -g "$ZID_GROUP" \
        -m \
        -s /usr/sbin/nologin \
        -c "ZID Authentication Service"
    echo "  Создан пользователь $ZID_USER"
fi
echo "✓ Пользователь и группа готовы"
echo ""

# Создание директорий
echo "📁 Создание директорий..."

for dir in "$ZID_HOME" "$ZID_LOG_DIR" "$ZID_RUN_DIR"; do
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
        echo "  Создана директория: $dir"
    else
        echo "  Директория существует: $dir"
    fi
    chown "$ZID_USER:$ZID_GROUP" "$dir"
    chmod 0750 "$dir"
done
echo "✓ Директории готовы"
echo ""

# Установка rc.d скрипта
echo "🔧 Установка rc.d скрипта..."
SCRIPT_DIR="$(dirname "$(realpath "$0")")"
RC_SCRIPT="$SCRIPT_DIR/zid.rc.d"

if [ ! -f "$RC_SCRIPT" ]; then
    echo "Ошибка: rc.d скрипт не найден: $RC_SCRIPT"
    exit 1
fi

install -m 0555 "$RC_SCRIPT" /usr/local/etc/rc.d/zid
echo "✓ RC.D скрипт установлен"
echo ""

# Создание конфиг-файла окружения
echo "⚙️  Создание конфиг-файла окружения..."
ENV_FILE="/usr/local/etc/zid.conf"

if [ ! -f "$ENV_FILE" ]; then
    cat > "$ENV_FILE" <<'EOF'
# ZID Authentication Service Configuration
# FreeBSD rc.d environment file

# Адрес и порт
ZID_HOST="127.0.0.1"
ZID_PORT="3000"

# Хранилища
SESSION_STORAGE="redis"
TICKET_STORAGE="redis"
CREDENTIALS_STORAGE="postgres"

# База данных PostgreSQL
DATABASE_URL="postgresql://zid:zid@localhost/zid"

# Redis
REDIS_URL="redis://localhost:6379"

# Telegram (опционально)
# TELEGRAM_BOT_USERNAME="your_bot_username"
# TELEGRAM_BOT_TOKEN="your_bot_token"

# Cookie
ZID_COOKIE_SECURE="false"

# Логирование
RUST_LOG="info"
RUST_BACKTRACE="1"

# Доверенные домены (comma-separated)
TRUSTED_DOMAINS="localhost:3000,example.com"
EOF
    echo "  Создан конфиг-файл: $ENV_FILE"
    chmod 0640 "$ENV_FILE"
    chown root:"$ZID_GROUP" "$ENV_FILE"
else
    echo "  Конфиг-файл уже существует: $ENV_FILE"
    echo "  Проверьте и обновите содержимое при необходимости"
fi
echo "✓ Конфиг-файл готов"
echo ""

# Итоговый статус
echo "=== Установка завершена ==="
echo ""
echo "📋 Следующие шаги:"
echo ""
echo "1. Проверьте конфигурацию (отредактируйте если нужно):"
echo "   nano /usr/local/etc/zid.conf"
echo ""
echo "2. Отредактируйте /etc/rc.conf для автозапуска (опционально):"
echo "   echo 'zid_enable=\"YES\"' >> /etc/rc.conf"
echo ""
echo "3. Запустите сервис:"
echo "   service zid start"
echo ""
echo "4. Проверьте статус:"
echo "   service zid status"
echo ""
echo "5. Просмотрите логи:"
echo "   tail -f /var/log/zid/zid.log"
echo ""
