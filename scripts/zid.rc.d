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
#
# zid_enable (bool):		Включить сервис. По умолчанию "NO".
# zid_user (str):		Пользователь для запуска. По умолчанию "zid".
# zid_group (str):		Группа. По умолчанию "zid".
# zid_env_file (str):		Файл с переменными окружения (см. rc.subr(8)).
#				По умолчанию "/usr/local/etc/zid/zid.conf".
# zid_config (str):		Устаревший синоним zid_env_file.
# zid_logfile (str):		Файл логов. По умолчанию "/var/log/zid/zid.log".
# zid_pidfile (str):		PID-файл. По умолчанию "/var/run/zid/zid.pid".
#

. /etc/rc.subr

name="zid"
rcvar=zid_enable

load_rc_config "$name"

: ${zid_enable:=NO}
: ${zid_user:=zid}
: ${zid_group:=zid}
: ${zid_config:=/usr/local/etc/zid/zid.conf}
: ${zid_env_file:=$zid_config}
: ${zid_logfile:=/var/log/zid/zid.log}
: ${zid_pidfile:=/var/run/zid/zid.pid}

command="/usr/local/bin/${name}"
pidfile="${zid_pidfile}"

start_cmd="${name}_start"
stop_cmd="${name}_stop"
restart_cmd="${name}_restart"

# Проверка наличия файла с переменными окружения
check_env_file()
{
	if [ ! -f "$zid_env_file" ]; then
		warn "Env file not found: $zid_env_file"
		return 1
	fi
	return 0
}

# Запуск сервиса (daemon без -p/-o, чтобы не блокировать: ZID не делает daemonize)
zid_start()
{
	echo "Starting ZID Authentication Service..."

	check_env_file || return 1

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

	# Запуск без -p/-o: daemon сразу возвращает управление.
	# Переменные из zid_env_file подгружаем в окружение процесса (set -a; . file; set +a).
	# PID и вывод в лог делаем в обёртке.
	/usr/sbin/daemon -u "$zid_user" sh -c 'set -a; . "'"$zid_env_file"'"; set +a; echo $$ > "'"$pidfile"'"; exec '"$command"' >> "'"$zid_logfile"'" 2>&1'

	RETVAL=$?
	if [ $RETVAL -eq 0 ]; then
		sleep 1
		[ -f "$pidfile" ] && echo "ZID started successfully (PID: $(cat "$pidfile"))"
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

# Показать конфигурацию
zid_showconfig()
{
	echo "ZID Configuration:"
	echo "  Env file: $zid_env_file"
	echo "  User: $zid_user"
	echo "  Group: $zid_group"
	echo "  Log file: $zid_logfile"
	echo "  PID file: $pidfile"
	echo ""

	if [ -f "$zid_env_file" ]; then
		echo "Environment variables:"
		grep -v '^#' "$zid_env_file" | grep -v '^$' | sed 's/^/  /'
	else
		echo "  Env file not found!"
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

extra_commands="status config logs"
zid_config_cmd="zid_showconfig"
zid_logs_cmd="zid_logs"
zid_status_cmd="zid_status"

run_rc_command "$1"
