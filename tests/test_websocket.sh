#!/bin/bash

# Cazino WebSocket Test Script
# Tests real-time updates via WebSocket

BASE_URL="http://localhost:3000"
WS_URL="ws://localhost:3000/ws"

echo "🔌 Cazino WebSocket Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if websocat is installed
if ! command -v websocat &> /dev/null; then
    echo "⚠️  websocat not found. Installing via brew..."
    echo ""
    echo "To install manually, run:"
    echo "  brew install websocat"
    echo ""
    echo "Or on Linux:"
    echo "  cargo install websocat"
    echo ""
    exit 1
fi

echo "✓ websocat found"
echo ""

# Create a market first
echo "1. Creating market via HTTP..."
CREATE_MARKET=$(curl -s -X POST "$BASE_URL/api/markets" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "WebSocket Test Market",
    "duration_hours": 24,
    "starting_balance": 1000
  }')

MARKET_ID=$(echo "$CREATE_MARKET" | jq -r '.market.id')
ADMIN_ID=$(echo "$CREATE_MARKET" | jq -r '.user.id')
INVITE_CODE=$(echo "$CREATE_MARKET" | jq -r '.invite_code')

echo "   Market ID: $MARKET_ID"
echo "   Admin ID: $ADMIN_ID"
echo ""

# Connect to WebSocket and subscribe
echo "2. Connecting to WebSocket..."
echo "   URL: $WS_URL"
echo ""

# Start WebSocket connection in background, show first 10 messages
(
    echo "{\"type\":\"subscribe\",\"market_id\":\"$MARKET_ID\"}"
    sleep 5  # Keep connection alive for 5 seconds
) | websocat -n --text "$WS_URL" > /tmp/ws_output.txt 2>&1 &

WS_PID=$!
echo "   WebSocket client started (PID: $WS_PID)"
echo ""

# Give WebSocket time to connect
sleep 1

# Now perform actions and see if we get WebSocket updates
echo "3. Performing actions to trigger WebSocket broadcasts..."
echo ""

echo "   a) Bob joins market..."
BOB_JOIN=$(curl -s -X POST "$BASE_URL/api/markets/$INVITE_CODE/join" \
  -H "Content-Type: application/json" \
  -d '{"display_name":"Bob","avatar":"🧑"}')
BOB_ID=$(echo "$BOB_JOIN" | jq -r '.user.id')
sleep 0.5

echo "   b) Opening market..."
curl -s -X POST "$BASE_URL/api/markets/$MARKET_ID/open/$ADMIN_ID" > /dev/null
sleep 0.5

echo "   c) Creating bet..."
CREATE_BET=$(curl -s -X POST "$BASE_URL/api/markets/$MARKET_ID/bets/$ADMIN_ID/create" \
  -H "Content-Type: application/json" \
  -d "{
    \"subject_user_id\":\"$BOB_ID\",
    \"description\":\"Will Bob win?\",
    \"resolution_criteria\":\"Test\",
    \"initial_odds\":\"1:1\",
    \"opening_wager\":100
  }")
BET_ID=$(echo "$CREATE_BET" | jq -r '.bet.id')
sleep 0.5

echo "   d) Approving bet..."
curl -s -X POST "$BASE_URL/api/bets/$BET_ID/approve/$ADMIN_ID" > /dev/null
sleep 0.5

echo "   e) Placing wager..."
curl -s -X POST "$BASE_URL/api/bets/$BET_ID/wager/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{"side":"YES","amount":100}' > /dev/null
sleep 0.5

echo "   f) Resolving bet..."
curl -s -X POST "$BASE_URL/api/bets/$BET_ID/resolve/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{"outcome":"YES"}' > /dev/null
sleep 0.5

echo ""
echo "4. Checking WebSocket messages received..."
echo ""

# Wait a bit more for messages to arrive
sleep 1

# Kill WebSocket client
kill $WS_PID 2>/dev/null
wait $WS_PID 2>/dev/null

# Check if we got any messages
if [ -f /tmp/ws_output.txt ]; then
    MSG_COUNT=$(wc -l < /tmp/ws_output.txt | tr -d ' ')

    if [ "$MSG_COUNT" -gt 0 ]; then
        echo "✅ Received $MSG_COUNT WebSocket message(s)!"
        echo ""
        echo "Sample messages:"
        head -n 5 /tmp/ws_output.txt | while read -r line; do
            echo "$line" | jq -C '.' 2>/dev/null || echo "$line"
        done

        if [ "$MSG_COUNT" -gt 5 ]; then
            echo ""
            echo "... and $((MSG_COUNT - 5)) more message(s)"
        fi

        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "✅ WebSocket real-time updates working!"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

        rm /tmp/ws_output.txt
        exit 0
    else
        echo "⚠️  No messages received from WebSocket"
        echo "    This might indicate an issue with the WebSocket connection"
        rm /tmp/ws_output.txt
        exit 1
    fi
else
    echo "❌ No WebSocket output file found"
    exit 1
fi
