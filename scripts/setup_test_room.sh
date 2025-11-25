#!/bin/bash

# Setup a test game room with code TEST99
# This script creates a market, adds test users, and opens it for betting
#
# Usage:
#   ./scripts/setup_test_room.sh [base_url]
#
# Example:
#   ./scripts/setup_test_room.sh http://localhost:3000

set -e

BASE_URL="${1:-http://localhost:3000}"
API_URL="$BASE_URL/api"

echo "🎲 Setting up test room TEST99..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if server is running
if ! curl -s "$BASE_URL/health" > /dev/null 2>&1; then
    echo "❌ Server is not running at $BASE_URL"
    echo "Please start the server first with: cargo run"
    exit 1
fi

# Note: The backend needs to support custom invite codes
# For now, we'll create a market and you can use whatever code it generates
# If you want TEST99 specifically, we'll need to modify the backend

echo "Creating test market with code TEST99..."
CREATE_RESPONSE=$(curl -s -X POST "$API_URL/markets" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Room",
    "admin_name": "admin",
    "duration_hours": 168,
    "starting_balance": 1000,
    "device_id": "test-device-admin",
    "invite_code": "TEST99"
  }')

# Check for errors
if echo "$CREATE_RESPONSE" | jq -e '.error' > /dev/null 2>&1; then
    ERROR=$(echo "$CREATE_RESPONSE" | jq -r '.error')
    echo "❌ Failed to create market: $ERROR"
    echo ""
    echo "💡 If TEST99 already exists, you can:"
    echo "   1. Delete cazino.db and restart the server"
    echo "   2. Or just join the existing TEST99 room"
    exit 1
fi

MARKET_ID=$(echo "$CREATE_RESPONSE" | jq -r '.market.id')
ADMIN_ID=$(echo "$CREATE_RESPONSE" | jq -r '.user.id')
INVITE_CODE=$(echo "$CREATE_RESPONSE" | jq -r '.invite_code')

echo "✅ Market created!"
echo "   Market ID: $MARKET_ID"
echo "   Admin ID: $ADMIN_ID"
echo "   Invite Code: $INVITE_CODE"
echo ""

# Add test users
echo "Adding test user: alice..."
ALICE_JOIN=$(curl -s -X POST "$API_URL/markets/$INVITE_CODE/join" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "alice",
    "avatar": "👩",
    "device_id": "test-device-alice"
  }')
ALICE_ID=$(echo "$ALICE_JOIN" | jq -r '.user.id')
echo "   Alice ID: $ALICE_ID"

echo "Adding test user: bob..."
BOB_JOIN=$(curl -s -X POST "$API_URL/markets/$INVITE_CODE/join" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "bob",
    "avatar": "🧑",
    "device_id": "test-device-bob"
  }')
BOB_ID=$(echo "$BOB_JOIN" | jq -r '.user.id')
echo "   Bob ID: $BOB_ID"

echo ""
echo "Opening market for betting..."
curl -s -X POST "$API_URL/markets/$MARKET_ID/open/$ADMIN_ID" > /dev/null

echo ""
echo "Creating test bet..."
CREATE_BET=$(curl -s -X POST "$API_URL/markets/$MARKET_ID/bets/$ADMIN_ID/create" \
  -H "Content-Type: application/json" \
  -d "{
    \"subject_user_id\": \"$ALICE_ID\",
    \"description\": \"@alice will win the game\",
    \"initial_odds\": \"1:1\",
    \"opening_wager\": 100
  }")
BET_ID=$(echo "$CREATE_BET" | jq -r '.bet.id')
echo "   Bet ID: $BET_ID"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Test room setup complete!"
echo ""
echo "📋 To join the room:"
echo "   1. Open: $BASE_URL"
echo "   2. Click 'Join Market'"
echo "   3. Enter invite code: $INVITE_CODE"
echo ""
echo "👥 Test users available:"
echo "   - admin (you)"
echo "   - alice"
echo "   - bob"
echo ""
echo "💡 Quick test URL:"
echo "   $BASE_URL?code=$INVITE_CODE"
echo ""
