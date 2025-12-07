#!/bin/bash

# Test script for ZID ticket verification flow
# This script:
# 1. Logs in and gets a ticket
# 2. Verifies the ticket
# 3. Tries to verify the same ticket again (should fail - one-time use)

set -e

BASE_URL="${ZID_URL:-http://localhost:5555}"
SERVICE_URL="http://localhost:3000"

echo "🧪 Testing ZID Ticket Verification Flow"
echo "========================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Step 0a: Test password mismatch validation${NC}"
echo "POST $BASE_URL/register (with mismatched passwords)"

MISMATCH_RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/register" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=testmismatch&password=secret&password_confirm=different')

# Extract body and status code
MISMATCH_BODY=$(echo "$MISMATCH_RESPONSE" | sed -e 's/HTTP_CODE\:.*//g')
MISMATCH_HTTP_CODE=$(echo "$MISMATCH_RESPONSE" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

if echo "$MISMATCH_BODY" | grep -q "Passwords do not match"; then
    echo -e "${GREEN}✅ Password mismatch validation working correctly${NC}"
else
    echo -e "${RED}❌ Password mismatch validation failed${NC}"
    exit 1
fi
echo ""

sleep 1

echo -e "${YELLOW}Step 0b: Register test user${NC}"
echo "POST $BASE_URL/register"

REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/register" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=admin&password=secret&password_confirm=secret')

# Check if registration was successful (look for "Registration Successful" in HTML)
if echo "$REGISTER_RESPONSE" | grep -q "Registration Successful"; then
    echo "Response: Registration successful (HTML page returned)"
else
    echo "Response: Registration may have failed or user already exists"
fi
echo ""

sleep 1

echo -e "${YELLOW}Step 1: Login and get ticket${NC}"
echo "POST $BASE_URL/login"

LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secret",
    "return_to": "'$SERVICE_URL'"
  }')

echo "Response: $LOGIN_RESPONSE"
echo ""

# Extract ticket from response
TICKET=$(echo "$LOGIN_RESPONSE" | jq -r '.ticket')

if [ "$TICKET" == "null" ] || [ -z "$TICKET" ]; then
  echo -e "${RED}❌ Failed to get ticket from login response${NC}"
  exit 1
fi

echo -e "${GREEN}✅ Got ticket: $TICKET${NC}"
echo ""

# Wait a moment
sleep 1

echo -e "${YELLOW}Step 2: Verify the ticket (first time - should succeed)${NC}"
echo "POST $BASE_URL/verify"

VERIFY_RESPONSE=$(curl -s -X POST "$BASE_URL/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "ticket": "'$TICKET'",
    "service": "'$SERVICE_URL'"
  }')

echo "Response: $VERIFY_RESPONSE"
echo ""

# Check if verification succeeded
SUCCESS=$(echo "$VERIFY_RESPONSE" | jq -r '.success')

if [ "$SUCCESS" == "true" ]; then
  USERNAME=$(echo "$VERIFY_RESPONSE" | jq -r '.username')
  USER_ID=$(echo "$VERIFY_RESPONSE" | jq -r '.user_id')
  SESSION_ID=$(echo "$VERIFY_RESPONSE" | jq -r '.session_id')

  echo -e "${GREEN}✅ Ticket verified successfully!${NC}"
  echo "   User: $USERNAME"
  echo "   User ID: $USER_ID"
  echo "   Session ID: $SESSION_ID"
else
  echo -e "${RED}❌ Ticket verification failed${NC}"
  exit 1
fi

echo ""

# Wait a moment
sleep 1

echo -e "${YELLOW}Step 3: Try to verify the same ticket again (should fail - one-time use)${NC}"
echo "POST $BASE_URL/verify"

VERIFY_RESPONSE_2=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "ticket": "'$TICKET'",
    "service": "'$SERVICE_URL'"
  }')

# Extract body and status code
BODY=$(echo "$VERIFY_RESPONSE_2" | sed -e 's/HTTP_CODE\:.*//g')
HTTP_CODE=$(echo "$VERIFY_RESPONSE_2" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

echo "Response: $BODY"
echo "HTTP Status: $HTTP_CODE"
echo ""

if [ "$HTTP_CODE" != "200" ]; then
  echo -e "${GREEN}✅ Ticket rejected (one-time use working correctly)${NC}"
else
  echo -e "${RED}❌ Ticket was accepted again - one-time use NOT working!${NC}"
  exit 1
fi

echo ""
echo -e "${YELLOW}Step 4: Test with wrong service URL${NC}"

# Register second test user
curl -s -X POST "$BASE_URL/register" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d 'username=test_user&password=secret&password_confirm=secret' > /dev/null

# Login again to get a new ticket
LOGIN_RESPONSE_2=$(curl -s -X POST "$BASE_URL/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "test_user",
    "password": "secret",
    "return_to": "'$SERVICE_URL'"
  }')

TICKET_2=$(echo "$LOGIN_RESPONSE_2" | jq -r '.ticket')
echo "New ticket: $TICKET_2"

# Try to verify with wrong service URL
WRONG_SERVICE="http://evil.com"
VERIFY_RESPONSE_3=$(curl -s -w "\nHTTP_CODE:%{http_code}" -X POST "$BASE_URL/verify" \
  -H "Content-Type: application/json" \
  -d '{
    "ticket": "'$TICKET_2'",
    "service": "'$WRONG_SERVICE'"
  }')

BODY_3=$(echo "$VERIFY_RESPONSE_3" | sed -e 's/HTTP_CODE\:.*//g')
HTTP_CODE_3=$(echo "$VERIFY_RESPONSE_3" | tr -d '\n' | sed -e 's/.*HTTP_CODE://')

echo "Response: $BODY_3"
echo "HTTP Status: $HTTP_CODE_3"
echo ""

if [ "$HTTP_CODE_3" != "200" ]; then
  echo -e "${GREEN}✅ Ticket rejected for wrong service URL${NC}"
else
  echo -e "${RED}❌ Ticket accepted with wrong service URL - security issue!${NC}"
  exit 1
fi

echo ""
echo "=========================================="
echo -e "${GREEN}🎉 All verification tests passed!${NC}"
echo "=========================================="
