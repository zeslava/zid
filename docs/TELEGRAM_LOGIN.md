# Telegram Login Integration

Full guide for integrating Telegram login with ZID.

## Contents

- [Quick Start](#quick-start)
- [Detailed Setup](#detailed-setup)
- [API](#api)
- [Security](#security)
- [Local Development](#local-development)
- [Troubleshooting](#troubleshooting)

## Quick Start

### 1. Create a Telegram bot

1. Open Telegram and find [@BotFather](https://t.me/botfather)
2. Send the `/newbot` command
3. Follow the instructions:
   - Enter a bot name (e.g., "My ZID Bot")
   - Enter a bot username (e.g., "my_zid_bot", must end with "_bot")
4. Save the **bot token** (looks like: `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`)

### 2. Set up the domain for Login Widget

Telegram Login Widget only works with public domains or localhost.

Send `/setdomain` to @BotFather:
```
/setdomain
→ Select your bot
→ Enter domain: localhost (for development) or your real domain
```

For production, specify your real domain without the protocol:
- `cas.example.com`
- `localhost` (development only)
- `https://cas.example.com` (incorrect — do not include the protocol)

### 3. Configure environment variables

Create a `.env` file in the project root:

```bash
# Bot token from @BotFather
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrsTUVwxyz

# Bot username WITHOUT the @ symbol (e.g., my_cas_bot)
TELEGRAM_BOT_USERNAME=my_cas_bot

# Automatically create new users on first Telegram login
# true - any Telegram user can log in
# false - only existing users can log in via Telegram
TELEGRAM_AUTO_REGISTER=true
```

### 4. Start the server

```bash
# With Docker Compose (recommended)
docker compose up -d

# Or locally
cargo run --release
```

### 5. Verify it works

Open in browser: http://localhost:5555/

You should see:
- The standard login form (username/password)
- An "OR" divider
- A "Login with Telegram" button

## Detailed Setup

### Environment Variables

| Variable | Required | Description | Example |
|----------|----------|-------------|---------|
| `TELEGRAM_BOT_TOKEN` | Yes* | Bot token from @BotFather | `123456789:ABC...` |
| `TELEGRAM_BOT_USERNAME` | Yes* | Bot username without @ | `my_cas_bot` |
| `TELEGRAM_AUTO_REGISTER` | No | Auto-registration | `true` (default) |

*\* If either variable is not set, the Telegram Login button will not be displayed*

### Operating Modes

#### Mode 1: Auto-registration (default)

```bash
TELEGRAM_AUTO_REGISTER=true
```

**Behavior:**
- Any Telegram user can log in
- An account is automatically created on first login
- Username in DB: `tg_{telegram_username}` or `tg_{telegram_id}`

**Suitable for:**
- Public services
- When simple login without pre-registration is needed

#### Mode 2: Existing users only

```bash
TELEGRAM_AUTO_REGISTER=false
```

**Behavior:**
- Only users with a linked Telegram ID can log in
- An error is returned when an unknown user attempts to log in
- An administrator must manually create users

**Suitable for:**
- Corporate systems
- When user list control is needed

### Database Migration

When using Docker Compose, the migration is applied automatically.

For manual application:

```bash
# Connect to PostgreSQL
psql -U postgres -d zid -h localhost -p 5432

# Apply the migration
\i migrations/002_add_telegram_support.sql
```

The migration adds the following fields to the `users` table:
- `telegram_id` (BIGINT, UNIQUE) — Telegram user ID
- `telegram_username` (VARCHAR) — @username in Telegram
- `telegram_first_name` (VARCHAR) — First name
- `telegram_last_name` (VARCHAR) — Last name
- `telegram_photo_url` (TEXT) — Avatar URL
- `telegram_auth_date` (BIGINT) — Last authentication time

## API

### POST /login/telegram

Endpoint for Telegram authentication. Called automatically by JavaScript code after successful login via the Telegram Widget.

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

**Response (success):**
```json
{
  "ticket": "ZID-7a3b9c2f8e1d4a5b6c7d8e9f0a1b2c3d",
  "redirect_url": "http://localhost:3000?ticket=ZID-7a3b9c2f8e1d4a5b6c7d8e9f0a1b2c3d"
}
```

**Response (error):**
```json
{
  "error": "Telegram auth verification failed: Hash mismatch"
}
```

### Error Codes

| Code | Description | Cause |
|------|-------------|-------|
| 401 | Unauthorized | Hash verification failed — Telegram signature check did not pass |
| 404 | Not Found | User not found (when `TELEGRAM_AUTO_REGISTER=false`) |
| 500 | Internal Server Error | TELEGRAM_BOT_TOKEN not configured or database error |

## Security

### Data Authenticity Verification

All data from Telegram is verified for authenticity per the [official documentation](https://core.telegram.org/widgets/login#checking-authorization):

1. **Building data_check_string:**
   - All fields (except `hash`) are collected as `key=value`
   - Sorted alphabetically
   - Joined with `\n`

2. **Computing secret_key:**
   ```
   secret_key = SHA256(bot_token)
   ```

3. **Computing hash:**
   ```
   hash = HMAC-SHA256(data_check_string, secret_key)
   ```

4. **Comparison:**
   - The computed hash is compared with the hash from Telegram
   - Constant-time comparison is used for protection against timing attacks

5. **Time check:**
   - `auth_date` must not be older than 24 hours
   - Protection against replay attacks

### What does this provide?

- Cannot forge data on behalf of another user
- Cannot reuse old authorization data
- Only real Telegram users can log in
- Bot token is known only to the server (not transmitted to the client)

### Security Recommendations

1. **Keep the bot token secret:**
   - Use a `.env` file (add to `.gitignore`)
   - Never commit the token to Git
   - In production use secrets management (Docker secrets, K8s secrets, etc.)

2. **Use HTTPS in production:**
   - Telegram requires HTTPS for Login Widget (except localhost)
   - Get an SSL certificate (Let's Encrypt is free)

3. **Configure TRUSTED_DOMAINS:**
   ```bash
   TRUSTED_DOMAINS=cas.example.com,app.example.com
   ```

4. **Regularly update dependencies:**
   ```bash
   cargo update
   ```

## Local Development

### Problem: Telegram requires a public domain

Telegram Login Widget **does not work** with IP addresses and internal domains (except `localhost`).

### Solution 1: ngrok (recommended for development)

```bash
# Install ngrok: https://ngrok.com/download

# Start a tunnel
ngrok http 5555

# Copy the public URL (e.g., https://abc123.ngrok.io)
# Configure in @BotFather via /setdomain
```

**Pros:**
- Quick and simple
- Automatic HTTPS
- Works from any network

**Cons:**
- URL changes on each restart (free tier)
- Requires updating the domain in @BotFather

### Solution 2: localhost (initial development only)

```bash
# In @BotFather: /setdomain → localhost
```

**Pros:**
- No external service needed
- Works offline

**Cons:**
- Works locally only (cannot test from mobile)
- Domain must be changed for production

### Solution 3: Tailscale (for teams)

```bash
# Install Tailscale: https://tailscale.com/download

# Enable MagicDNS and HTTPS certificates
# Your machine will be available as: your-machine.tail-xxxxx.ts.net

# In @BotFather: /setdomain → your-machine.tail-xxxxx.ts.net
```

**Pros:**
- Permanent domain
- Automatic HTTPS
- Accessible to the whole team
- Works from any network

**Cons:**
- Requires Tailscale installation

## Troubleshooting

### Telegram button does not appear

**Cause:** Environment variables not configured.

**Solution:**
1. Verify that `TELEGRAM_BOT_TOKEN` and `TELEGRAM_BOT_USERNAME` are set
2. Check logs:
   ```bash
   docker compose logs zid-app
   ```
3. Restart the container:
   ```bash
   docker compose restart zid-app
   ```

### "Bot domain invalid"

**Cause:** Domain not configured in @BotFather.

**Solution:**
1. Send `/setdomain` to @BotFather
2. Select your bot
3. Enter the domain (without `https://`):
   - For development: `localhost`
   - For ngrok: `abc123.ngrok.io`
   - For production: `cas.example.com`

### "Hash verification failed"

**Cause 1:** Incorrect bot token.

**Solution:**
- Verify that `TELEGRAM_BOT_TOKEN` is correct
- Copy the token from @BotFather again

**Cause 2:** Server time out of sync.

**Solution:**
```bash
# Check server time
date

# Sync time (Linux)
sudo ntpdate -s time.nist.gov
```

### "Auth data is too old"

**Cause:** Data is older than 24 hours (or server time is incorrect).

**Solution:**
1. Check server time
2. Try logging in again (do not use cached data)

### "User not found" (with TELEGRAM_AUTO_REGISTER=false)

**Cause:** User does not exist in the database.

**Solution:**

**Option 1:** Enable auto-registration:
```bash
TELEGRAM_AUTO_REGISTER=true
```

**Option 2:** Create the user manually:
```sql
-- Connect to the database
psql -U postgres -d zid -h localhost -p 5432

-- Create the user
INSERT INTO users (id, username, telegram_id, telegram_username, telegram_first_name)
VALUES (
    gen_random_uuid()::text,
    'johndoe',
    123456789,  -- Telegram user ID
    'johndoe',  -- @username
    'John'
);
```

### CORS errors in the browser

**Cause:** Telegram Widget is loaded from a different domain.

**Solution:** This is normal. CORS errors from Telegram Widget can be ignored — they do not affect functionality.

### Does not work on mobile device

**Cause:** `localhost` is not accessible from a mobile device.

**Solution:** Use ngrok or Tailscale (see the "Local Development" section).

## Usage Examples

### Example 1: Public service with auto-registration

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=my_cas_bot
TELEGRAM_AUTO_REGISTER=true
TRUSTED_DOMAINS=cas.example.com,app1.example.com,app2.example.com
```

**Result:**
- Any Telegram user can log in
- An account is created on first login
- The ticket can be used for all domains in `TRUSTED_DOMAINS`

### Example 2: Corporate system with access control

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=company_auth_bot
TELEGRAM_AUTO_REGISTER=false
TRUSTED_DOMAINS=*.company.com
```

**Result:**
- Only users with a linked Telegram ID can log in
- HR or admin creates users in the database
- All subdomains of company.com are supported

### Example 3: Hybrid authentication

```bash
# .env
TELEGRAM_BOT_TOKEN=123456789:ABC...
TELEGRAM_BOT_USERNAME=my_bot
TELEGRAM_AUTO_REGISTER=true
```

**Users can:**
1. Register via the form (username + password)
2. Log in via Telegram (a new account is created)
3. Link Telegram to an existing account (TODO: feature)

## Useful Links

- [Official Telegram Login Widget documentation](https://core.telegram.org/widgets/login)
- [Authorization verification](https://core.telegram.org/widgets/login#checking-authorization)
- [@BotFather](https://t.me/botfather) — bot creation and configuration
- [ngrok](https://ngrok.com/) — tunneling for local development
- [Tailscale](https://tailscale.com/) — VPN for teams
