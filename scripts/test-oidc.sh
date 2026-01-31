#!/bin/bash
# Тест OIDC/OAuth 2.0 (discovery, client_credentials, jwks, userinfo).
# Сервер должен быть запущен с OIDC_ENABLED=true и настроенными ключами и клиентами.
# Использование: ./scripts/test-oidc.sh [BASE_URL]
# Пример: ZID_URL=http://localhost:5555 ./scripts/test-oidc.sh

set -e

BASE="${ZID_URL:-http://localhost:5555}"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "OIDC/OAuth 2.0 тесты"
echo "===================="
echo "BASE: $BASE"
echo ""

# 1. Discovery
echo -e "${YELLOW}1. Discovery${NC}"
echo "GET $BASE/.well-known/openid-configuration"
DISC=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$BASE/.well-known/openid-configuration")
DISC_BODY=$(echo "$DISC" | sed -e 's/HTTP_CODE\:.*//g')
DISC_CODE=$(echo "$DISC" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

if [ "$DISC_CODE" != "200" ]; then
  echo -e "${RED}Ошибка: ожидался 200, получен $DISC_CODE${NC}"
  echo "$DISC_BODY" | head -5
  exit 1
fi
if echo "$DISC_BODY" | grep -q '"issuer"'; then
  echo -e "${GREEN}OK: issuer и endpoints присутствуют${NC}"
else
  echo -e "${RED}Ошибка: ответ не похож на discovery${NC}"
  exit 1
fi
echo ""

# 2. Client Credentials
echo -e "${YELLOW}2. Client Credentials (token)${NC}"
echo "POST $BASE/oauth/token (grant_type=client_credentials)"
TOKEN_RESP=$(curl -s -X POST "$BASE/oauth/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=service-m2m" \
  -d "client_secret=machine-secret")

if echo "$TOKEN_RESP" | grep -q '"access_token"'; then
  echo -e "${GREEN}OK: access_token получен${NC}"
  ACCESS_TOKEN=$(echo "$TOKEN_RESP" | jq -r '.access_token')
  if [ "$ACCESS_TOKEN" = "null" ] || [ -z "$ACCESS_TOKEN" ]; then
    echo -e "${RED}Не удалось извлечь access_token (нужен jq)${NC}"
  fi
else
  echo -e "${RED}Ошибка: ответ без access_token${NC}"
  echo "$TOKEN_RESP"
  exit 1
fi
echo ""

# 3. JWKS
echo -e "${YELLOW}3. JWKS${NC}"
echo "GET $BASE/oauth/jwks"
JWKS=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$BASE/oauth/jwks")
JWKS_BODY=$(echo "$JWKS" | sed -e 's/HTTP_CODE\:.*//g')
JWKS_CODE=$(echo "$JWKS" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

if [ "$JWKS_CODE" != "200" ]; then
  echo -e "${RED}Ошибка: ожидался 200, получен $JWKS_CODE${NC}"
  exit 1
fi
if echo "$JWKS_BODY" | grep -q '"keys"'; then
  echo -e "${GREEN}OK: keys присутствуют${NC}"
else
  echo -e "${RED}Ошибка: ответ не похож на JWKS${NC}"
  exit 1
fi
echo ""

# 4. UserInfo (с токеном client_credentials — sub будет client_id)
if [ -n "$ACCESS_TOKEN" ] && [ "$ACCESS_TOKEN" != "null" ]; then
  echo -e "${YELLOW}4. UserInfo (Bearer token)${NC}"
  echo "GET $BASE/oauth/userinfo"
  UI_RESP=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$BASE/oauth/userinfo" \
    -H "Authorization: Bearer $ACCESS_TOKEN")
  UI_BODY=$(echo "$UI_RESP" | sed -e 's/HTTP_CODE\:.*//g')
  UI_CODE=$(echo "$UI_RESP" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

  if [ "$UI_CODE" = "200" ]; then
    echo -e "${GREEN}OK: UserInfo 200${NC}"
    if echo "$UI_BODY" | grep -q '"sub"'; then
      echo "  sub: $(echo "$UI_BODY" | jq -r '.sub')"
    fi
  else
    echo -e "${RED}Ошибка: ожидался 200, получен $UI_CODE${NC}"
    echo "$UI_BODY"
    exit 1
  fi
else
  echo -e "${YELLOW}4. UserInfo — пропуск (нет access_token, установите jq)${NC}"
fi
echo ""

echo -e "${GREEN}Все проверки OIDC пройдены.${NC}"
echo "Authorization Code flow тестируйте вручную по docs/OIDC_TESTING.md"
