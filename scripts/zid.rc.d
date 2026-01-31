#!/bin/sh
#
# PROVIDE: zid
# REQUIRE: NETWORKING DAEMON
# BEFORE: LOGIN
# KEYWORD: nojail
#
# ZID Authentication Service
# FreeBSD rc.d service script
#
# Добавьте в /etc/rc.conf для автозапуска:
#   zid_enable="YES"
#   zid_user="zid"          # (опционально, по умолчанию: zid)
#   zid_group="zid"         # (опционально, по умолчанию: zid)
#   zid_config="/usr/local/etc/zid.conf"  # (опционально, по умолчанию: /usr/local/etc/zid.conf)
#   zid_logfile="/var/log/zid/zid.log"  # (опционально)
#   zid_pidfile="/var/run/zid/zid.pid"  # (опционально)
#

. /etc/rc.subr

name="zid"
rcvar=zid_enable

# Пути по умолчанию
: ${zid_enable:=NO}
: ${zid_user:=zid}
: ${zid_group:=zid}
: ${zid_config:=/usr/local/etc/zid.conf}
: ${zid_logfile:=/var/log/zid/zid.log}
: ${zid_pidfile:=/var/run/zid/zid.pid}

command="/usr/local/bin/${name}"
pidfile="${zid_pidfile}"

# Проверка наличия конфиг-файла
check_config()
{
	if [ ! -f "$zid_config" ]; then
		warn "Config file not found: $zid_config"
		return 1
	fi
	return 0
}

# Загрузка переменных окружения
load_env()
{
	if [ -f "$zid_config" ]; then
		# Источник конфига (только переменные)
		set -a
		. "$zid_config"
		set +a
	fi
}

# Запуск сервиса
zid_start()
{
	echo "Starting ZID Authentication Service..."

	check_config || return 1
	load_env

	# Создание директории для PID файла если не существует
	if [ ! -d "$(dirname "$pidfile")" ]; then
		mkdir -p "$(dirname "$pidfile")"
		chown "$zid_user:$zid_group" "$(dirname "$pidfile")"
	fi

	# Создание директории для логов если не существует
	if [ ! -d "$(dirname "$zid_logfile")" ]; then
		mkdir -p "$(dirname "$zid_logfile")"
		chown "$zid_user:$zid_group" "$(dirname "$zid_logfile")"
	fi

	# Запуск приложения
	/usr/sbin/daemon \
		-p "$pidfile" \
		-u "$zid_user" \
		-o "$zid_logfile" \
		-S \
		"$command"

	RETVAL=$?
	if [ $RETVAL -eq 0 ]; then
		echo "ZID started successfully (PID: $(cat "$pidfile" 2>/dev/null))"
	fi
	return $RETVAL
}

# Остановка сервиса
zid_stop()
{
	echo "Stopping ZID Authentication Service..."

	if [ -f "$pidfile" ]; then
		pid=$(cat "$pidfile")
		if kill -0 "$pid" 2>/dev/null; then
			kill "$pid"
			# Ждем завершения процесса (максимум 10 секунд)
			for i in 1 2 3 4 5 6 7 8 9 10; do
				if ! kill -0 "$pid" 2>/dev/null; then
					rm -f "$pidfile"
					echo "ZID stopped successfully"
					return 0
				fi
				sleep 1
			done
			# Если процесс не завершился, убиваем жестко
			echo "Force killing ZID (PID: $pid)"
			kill -9 "$pid"
			rm -f "$pidfile"
		else
			echo "ZID is not running (stale PID file)"
			rm -f "$pidfile"
		fi
	else
		echo "ZID is not running"
	fi
	return 0
}

# Перезагрузка сервиса
zid_restart()
{
	zid_stop
	sleep 1
	zid_start
}

# Проверка статуса
zid_status()
{
	if [ -f "$pidfile" ]; then
		pid=$(cat "$pidfile")
		if kill -0 "$pid" 2>/dev/null; then
			echo "ZID is running (PID: $pid)"
			return 0
		else
			echo "ZID is not running (stale PID file: $pidfile)"
			return 1
		fi
	else
		echo "ZID is not running"
		return 1
	fi
}

# Проверка конфигурации
zid_config()
{
	echo "ZID Configuration:"
	echo "  Config file: $zid_config"
	echo "  User: $zid_user"
	echo "  Group: $zid_group"
	echo "  Log file: $zid_logfile"
	echo "  PID file: $pidfile"
	echo ""
	
	if [ -f "$zid_config" ]; then
		echo "Environment variables:"
		grep -v '^#' "$zid_config" | grep -v '^$' | sed 's/^/  /'
	else
		echo "  Config file not found!"
	fi
}

# Просмотр логов
zid_logs()
{
	if [ -f "$zid_logfile" ]; then
		tail -f "$zid_logfile"
	else
		echo "Log file not found: $zid_logfile"
		return 1
	fi
}

# Установка пользовательских команд
extra_commands="status config logs"
zid_config_cmd="zid_config"
zid_logs_cmd="zid_logs"
zid_status_cmd="zid_status"

# Запуск основной функции rc.subr
load_rc_config "$name"
run_rc_command "$1"
