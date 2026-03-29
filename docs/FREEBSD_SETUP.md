# ZID on FreeBSD — Installation Guide

This guide describes installing and configuring the ZID Authentication Service on FreeBSD.

## Requirements

- FreeBSD 12.0 or newer
- Rust (for building) or a pre-built binary
- PostgreSQL 12+
- Redis 6.0+

## Quick Installation

### 1. Build the project

```bash
cd /path/to/zid
cargo build --release
```

### 2. Run the setup script

```bash
sudo sh ./scripts/setup-freebsd.sh ./target/release/zid
```

The script automatically:
- Creates a `zid` user and `zid` group
- Installs the binary to `/usr/local/bin/zid`
- Installs the rc.d script to `/usr/local/etc/rc.d/zid`
- Creates the necessary directories and configuration files

### 3. Configuration

Edit the environment variables file (default `/usr/local/etc/zid/zid.env`):

```bash
sudo nano /usr/local/etc/zid/zid.env
```

Make sure the environment variables are correct (the application reads `SERVER_HOST`, `SERVER_PORT`, `POSTGRES_*`, `REDIS_URL`, etc. — see the section below).

### 4. Start

**One-time start** (the command returns immediately, the service runs in the background):
```bash
sudo service zid start
```

**Auto-start on boot:**
```bash
echo 'zid_enable="YES"' | sudo tee -a /etc/rc.conf
sudo service zid start
```

## Cross-compilation from Linux (amd64)

To build a binary for FreeBSD aarch64 on a Linux amd64 machine (without FreeBSD access):

**Requirements:** Docker, Rust (rustup), [cross](https://github.com/cross-rs/cross). The target `aarch64-unknown-freebsd` is not supported on stable on the host (Linux) — the build runs inside a cross Docker image with nightly and build-std.

```bash
# One-time: install cross
cargo install cross

# Build (uses nightly with build-std inside the container)
task cross-freebsd-aarch64
```

Artifact: `./target/aarch64-unknown-freebsd/release/zid`. Copy it to FreeBSD (e.g., via `scp`) and install using the standard procedure:

```bash
# On FreeBSD after copying the binary
sudo sh ./scripts/setup-freebsd.sh ./target/aarch64-unknown-freebsd/release/zid
```

The project root contains a `Cross.toml` with the image for the `aarch64-unknown-freebsd` target. If the image `ghcr.io/cross-rs/aarch64-unknown-freebsd:latest` is not found during the build, build it from the cross-rs repository:

```bash
git clone --depth 1 https://github.com/cross-rs/cross && cd cross/docker && docker build -f Dockerfile.aarch64-unknown-freebsd -t ghcr.io/cross-rs/aarch64-unknown-freebsd:latest .
```

If Docker is unavailable, manual setup is required: a cross-linker and FreeBSD aarch64 sysroot, with the linker specified in `.cargo/config.toml` for the `aarch64-unknown-freebsd` target.

## Management Commands

| Command | Description |
|---------|-------------|
| `sudo service zid start` | Start the service |
| `sudo service zid stop` | Stop the service |
| `sudo service zid restart` | Restart the service |
| `sudo service zid status` | Check status |
| `sudo service zid config` | Show configuration |
| `sudo service zid logs` | View logs (tail -f) |

## File Structure

```
/usr/local/bin/zid                 # Service binary
/usr/local/etc/rc.d/zid            # RC.D script (service management)
/usr/local/etc/zid/                # ZID configuration directory
  zid.env                         # Environment variables file (zid_env_file)
  oidc_clients.yaml                # OIDC: clients (if OIDC is enabled)
  oidc_jwt_private.pem             # OIDC: JWT signing key
  oidc_jwt_public.pem              # OIDC: public key (JWKS)
/var/lib/zid/                      # Home directory for zid user
/var/log/zid/zid.log               # Service logs
/var/run/zid/zid.pid               # PID file
```

## rc.conf Configuration

Add to `/etc/rc.conf` (for auto-start):

```bash
# Main parameters
zid_enable="YES"

# Optional parameters (default values)
zid_user="zid"                           # Unix user
zid_group="zid"                          # Unix group
zid_env_file="/usr/local/etc/zid/zid.env"   # Environment variables file (see rc.subr(8))
zid_logfile="/var/log/zid/zid.log"       # Log file
zid_pidfile="/var/run/zid/zid.pid"       # PID file
```

The `zid_env_file` variable is standard for rc.subr(8): rc.d automatically picks up environment variables from it at startup. The deprecated alias `zid_config` is also supported for compatibility.

## Environment Variables (in zid_env_file)

```bash
# Address and port (the application reads SERVER_HOST, SERVER_PORT)
SERVER_HOST="0.0.0.0"
SERVER_PORT="5555"

# Storage (postgres by default, redis as alternative)
SESSION_STORAGE="postgres"
TICKET_STORAGE="postgres"
CREDENTIALS_STORAGE="postgres"

# PostgreSQL database
DATABASE_URL="postgresql://user:pass@localhost/zid"

# Redis
REDIS_URL="redis://localhost:6379"

# Telegram (optional)
TELEGRAM_BOT_USERNAME="your_bot"
TELEGRAM_BOT_TOKEN="your_token"

# Security
ZID_COOKIE_SECURE="false"           # true for HTTPS, false for local development

# Logging
RUST_LOG="info"                     # Levels: trace, debug, info, warn, error
RUST_BACKTRACE="1"                  # Backtrace on panic

# Trusted domains (for return_to redirects)
TRUSTED_DOMAINS="localhost:3000,app.example.com,api.example.com"
```

## Dependencies

### PostgreSQL

Make sure PostgreSQL is running and the database is accessible:

```bash
# Check status
sudo service postgresql status

# Create DB and user (if they don't exist)
sudo -u postgres createuser -P zid
sudo -u postgres createdb -O zid zid

# Apply migrations
cd /path/to/zid
sqlx migrate run --database-url="postgresql://zid:pass@localhost/zid"
```

### Redis

Make sure Redis is running:

```bash
# Check status
sudo service redis status

# Start if disabled
sudo service redis start

# Add to /etc/rc.conf for auto-start
echo 'redis_enable="YES"' | sudo tee -a /etc/rc.conf
```

## Nginx Integration (reverse proxy)

Example Nginx config:

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

Add the config to `/usr/local/etc/nginx/conf.d/`:

```bash
sudo cp zid-nginx.conf /usr/local/etc/nginx/conf.d/
sudo service nginx reload
```

## Monitoring and Logging

### Viewing logs

```bash
# Real-time
sudo tail -f /var/log/zid/zid.log

# Or via the rc.d command
sudo service zid logs
```

### Log rotation

Add to `/etc/newsyslog.conf`:

```
/var/log/zid/zid.log   zid:zid   644  7  *  @daily  Z
```

### Health monitoring

Check the health-check endpoint:

```bash
curl http://localhost:3000/health
```

## Troubleshooting

### Service does not start

1. Check logs:
   ```bash
   sudo tail -50 /var/log/zid/zid.log
   ```

2. Check configuration:
   ```bash
   sudo service zid config
   ```

3. Make sure dependencies are running:
   ```bash
   sudo service postgresql status
   sudo service redis status
   ```

### Database connection error

```bash
# Check DATABASE_URL in the zid_env_file (default /usr/local/etc/zid/zid.env)
sudo cat /usr/local/etc/zid/zid.env | grep DATABASE_URL

# Test the connection
psql "postgresql://zid:pass@localhost/zid" -c "SELECT 1"
```

### Permission error

Check file permissions:

```bash
ls -la /var/log/zid/
ls -la /var/run/zid/
ls -la /usr/local/etc/zid/zid.env
```

Fix if needed:

```bash
sudo chown zid:zid /var/log/zid/
sudo chown zid:zid /var/run/zid/
```

### Port check

```bash
# Check that port 3000 is listening
sudo sockstat -l | grep 3000

# If there is a conflict, change SERVER_PORT in the zid_env_file
```

## Updating

### Updating the binary

```bash
# 1. Build the new version
cd /path/to/zid
git pull
cargo build --release

# 2. Reinstall with the new binary
sudo sh ./scripts/setup-freebsd.sh ./target/release/zid

# 3. Restart the service
sudo service zid restart

# 4. Check status
sudo service zid status
```

### Rollback

```bash
sudo service zid stop
# Restore the old binary manually
sudo service zid start
```

## Uninstallation

```bash
# 1. Stop the service
sudo service zid stop

# 2. Remove from auto-start
sudo sed -i '' '/zid_enable/d' /etc/rc.conf

# 3. Remove files
sudo rm /usr/local/bin/zid
sudo rm /usr/local/etc/rc.d/zid
sudo rm /usr/local/etc/zid/zid.env

# 4. Remove user and directories (optional)
sudo pw userdel zid
sudo rm -rf /var/lib/zid /var/log/zid /var/run/zid
```

## See Also

- [Telegram Login](TELEGRAM_LOGIN.md)
