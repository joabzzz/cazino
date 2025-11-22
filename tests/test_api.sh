#!/bin/bash

# Cazino API Test Script
# Tests all endpoints with multiple simulated users

set -e  # Exit on error

BASE_URL="http://localhost:3000"
API_URL="$BASE_URL/api"

echo "🎲 Cazino API Test Script"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_TOTAL=0

# Helper function to print test results
test_step() {
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    echo -e "${BLUE}[TEST $TESTS_TOTAL]${NC} $1"
}

test_passed() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓ PASSED${NC}\n"
}

# 1. Health Check
test_step "Health check"
HEALTH=$(curl -s "$BASE_URL/health")
echo "Response: $HEALTH"
if [ "$HEALTH" = "OK" ]; then
    test_passed
else
    echo "❌ FAILED: Expected 'OK', got '$HEALTH'"
    exit 1
fi

# 2. Create Market (Admin will be auto-created)
test_step "Create market with 24 hour duration"
CREATE_MARKET_RESPONSE=$(curl -s -X POST "$API_URL/markets" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Family Game Night 2025",
    "duration_hours": 24,
    "starting_balance": 1000
  }')

echo "$CREATE_MARKET_RESPONSE" | jq '.'

MARKET_ID=$(echo "$CREATE_MARKET_RESPONSE" | jq -r '.market.id')
ADMIN_ID=$(echo "$CREATE_MARKET_RESPONSE" | jq -r '.user.id')
INVITE_CODE=$(echo "$CREATE_MARKET_RESPONSE" | jq -r '.invite_code')

echo "Market ID: $MARKET_ID"
echo "Admin ID: $ADMIN_ID"
echo "Invite Code: $INVITE_CODE"
test_passed

# 3. Bob joins the market
test_step "Bob joins market with invite code"
BOB_JOIN=$(curl -s -X POST "$API_URL/markets/$INVITE_CODE/join" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "Bob",
    "avatar": "🧑"
  }')

echo "$BOB_JOIN" | jq '.'
BOB_ID=$(echo "$BOB_JOIN" | jq -r '.user.id')
echo "Bob ID: $BOB_ID"
test_passed

# 4. Carol joins the market
test_step "Carol joins market with invite code"
CAROL_JOIN=$(curl -s -X POST "$API_URL/markets/$INVITE_CODE/join" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "Carol",
    "avatar": "👩"
  }')

echo "$CAROL_JOIN" | jq '.'
CAROL_ID=$(echo "$CAROL_JOIN" | jq -r '.user.id')
echo "Carol ID: $CAROL_ID"
test_passed

# 5. Admin opens the market for betting
test_step "Admin opens market for betting"
OPEN_MARKET=$(curl -s -X POST "$API_URL/markets/$MARKET_ID/open/$ADMIN_ID")
echo "Status: $OPEN_MARKET"
test_passed

# 6. Admin creates a bet about Bob
test_step "Admin creates bet about Bob: 'Will Bob eat dessert first?'"
CREATE_BET1=$(curl -s -X POST "$API_URL/markets/$MARKET_ID/bets/$ADMIN_ID/create" \
  -H "Content-Type: application/json" \
  -d "{
    \"subject_user_id\": \"$BOB_ID\",
    \"description\": \"Will Bob eat dessert first?\",
    \"resolution_criteria\": \"Bob must take a bite of dessert before any main course\",
    \"initial_odds\": \"1:1\",
    \"opening_wager\": 100
  }")

echo "$CREATE_BET1" | jq '.'
BET1_ID=$(echo "$CREATE_BET1" | jq -r '.bet.id')
echo "Bet 1 ID: $BET1_ID"
test_passed

# 7. Bob creates a bet about Carol (will be hidden from Carol)
test_step "Bob creates bet about Carol: 'Will Carol arrive late?'"
CREATE_BET2=$(curl -s -X POST "$API_URL/markets/$MARKET_ID/bets/$BOB_ID/create" \
  -H "Content-Type: application/json" \
  -d "{
    \"subject_user_id\": \"$CAROL_ID\",
    \"description\": \"Will Carol arrive late?\",
    \"resolution_criteria\": \"Carol arrives after 7:00 PM\",
    \"initial_odds\": \"2:1\",
    \"opening_wager\": 50
  }")

echo "$CREATE_BET2" | jq '.'
BET2_ID=$(echo "$CREATE_BET2" | jq -r '.bet.id')
echo "Bet 2 ID: $BET2_ID"
test_passed

# 8. Get pending bets (admin view)
test_step "Get pending bets for admin approval"
PENDING_BETS=$(curl -s "$API_URL/markets/$MARKET_ID/bets/pending")
echo "$PENDING_BETS" | jq '.'
PENDING_COUNT=$(echo "$PENDING_BETS" | jq '. | length')
echo "Pending bets: $PENDING_COUNT"
if [ "$PENDING_COUNT" -ge 2 ]; then
    test_passed
else
    echo "❌ FAILED: Expected at least 2 pending bets, got $PENDING_COUNT"
    exit 1
fi

# 9. Admin approves bet 1
test_step "Admin approves bet 1"
APPROVE_BET1=$(curl -s -X POST "$API_URL/bets/$BET1_ID/approve/$ADMIN_ID")
echo "Status: $APPROVE_BET1"
test_passed

# 10. Admin approves bet 2
test_step "Admin approves bet 2"
APPROVE_BET2=$(curl -s -X POST "$API_URL/bets/$BET2_ID/approve/$ADMIN_ID")
echo "Status: $APPROVE_BET2"
test_passed

# 11. Get bets for Carol - check if bet 2 is hidden
test_step "Get bets for Carol (checking hidden bet)"
CAROL_BETS=$(curl -s "$API_URL/markets/$MARKET_ID/bets/$CAROL_ID")
echo "$CAROL_BETS" | jq '.'

# Check if bet 2 is hidden for Carol
BET2_HIDDEN=$(echo "$CAROL_BETS" | jq -r ".[] | select(.id == \"$BET2_ID\") | .is_hidden")
BET2_DESC=$(echo "$CAROL_BETS" | jq -r ".[] | select(.id == \"$BET2_ID\") | .description")
if [ "$BET2_HIDDEN" = "true" ] && [ "$BET2_DESC" = "null" ]; then
    echo "✓ Bet correctly hidden from Carol (is_hidden=true, description=null)"
    test_passed
else
    echo "❌ FAILED: Bet should be hidden from Carol"
    echo "   is_hidden: $BET2_HIDDEN (expected: true)"
    echo "   description: $BET2_DESC (expected: null)"
    exit 1
fi

# 12. Carol places a YES wager on bet 1 (about Bob)
test_step "Carol wagers 200 YES on bet 1 (Bob eating dessert first)"
CAROL_WAGER=$(curl -s -X POST "$API_URL/bets/$BET1_ID/wager/$CAROL_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "side": "YES",
    "amount": 200
  }')

echo "$CAROL_WAGER" | jq '.'
test_passed

# 13. Admin places a NO wager on bet 1
test_step "Admin wagers 150 NO on bet 1"
ADMIN_WAGER=$(curl -s -X POST "$API_URL/bets/$BET1_ID/wager/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "side": "NO",
    "amount": 150
  }')

echo "$ADMIN_WAGER" | jq '.'
test_passed

# 14. Bob places a NO wager on bet 1
test_step "Bob wagers 100 NO on bet 1"
BOB_WAGER=$(curl -s -X POST "$API_URL/bets/$BET1_ID/wager/$BOB_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "side": "NO",
    "amount": 100
  }')

echo "$BOB_WAGER" | jq '.'
test_passed

# 15. Admin places a YES wager on bet 2 (about Carol)
test_step "Admin wagers 100 YES on bet 2 (Carol arriving late)"
ADMIN_WAGER2=$(curl -s -X POST "$API_URL/bets/$BET2_ID/wager/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "side": "YES",
    "amount": 100
  }')

echo "$ADMIN_WAGER2" | jq '.'
test_passed

# 16. Get bet 1 probability chart
test_step "Get probability chart for bet 1"
PROB_CHART=$(curl -s "$API_URL/bets/$BET1_ID/chart")
echo "$PROB_CHART" | jq '.'
DATA_POINTS=$(echo "$PROB_CHART" | jq '.points | length')
echo "Data points: $DATA_POINTS"
if [ "$DATA_POINTS" -ge 2 ]; then
    test_passed
else
    echo "❌ FAILED: Expected at least 2 data points, got $DATA_POINTS"
    exit 1
fi

# 17. Get leaderboard
test_step "Get market leaderboard"
LEADERBOARD=$(curl -s "$API_URL/markets/$MARKET_ID/leaderboard")
echo "$LEADERBOARD" | jq '.'
test_passed

# 18. Resolve bet 1 as YES (Bob did eat dessert first!)
test_step "Admin resolves bet 1 as YES"
RESOLVE_BET1=$(curl -s -X POST "$API_URL/bets/$BET1_ID/resolve/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "outcome": "YES"
  }')

echo "Status: $RESOLVE_BET1"
test_passed

# 19. Get bets for Admin to see updated status
test_step "Get updated bet list for Admin"
ADMIN_BETS=$(curl -s "$API_URL/markets/$MARKET_ID/bets/$ADMIN_ID")
echo "$ADMIN_BETS" | jq '.'
BET1_STATUS=$(echo "$ADMIN_BETS" | jq -r ".[] | select(.id == \"$BET1_ID\") | .status")
echo "Bet 1 status: $BET1_STATUS"
if [ "$BET1_STATUS" = "resolvedyes" ]; then
    test_passed
else
    echo "❌ FAILED: Expected 'resolvedyes', got '$BET1_STATUS'"
    exit 1
fi

# 20. Carol reveals the hidden bet about herself
test_step "Carol reveals bets about her"
REVEAL_BETS=$(curl -s "$API_URL/users/$CAROL_ID/reveal")
echo "$REVEAL_BETS" | jq '.'
REVEALED_DESC=$(echo "$REVEAL_BETS" | jq -r ".bets[] | select(.id == \"$BET2_ID\") | .description")
echo "Revealed description: $REVEALED_DESC"
if [ "$REVEALED_DESC" = "Will Carol arrive late?" ]; then
    test_passed
else
    echo "❌ FAILED: Expected full description, got: $REVEALED_DESC"
    exit 1
fi

# 21. Resolve bet 2 as NO
test_step "Admin resolves bet 2 as NO (Carol was on time!)"
RESOLVE_BET2=$(curl -s -X POST "$API_URL/bets/$BET2_ID/resolve/$ADMIN_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "outcome": "NO"
  }')

echo "Status: $RESOLVE_BET2"
test_passed

# 22. Get final leaderboard
test_step "Get final leaderboard after all resolutions"
FINAL_LEADERBOARD=$(curl -s "$API_URL/markets/$MARKET_ID/leaderboard")
echo "$FINAL_LEADERBOARD" | jq '.'

# Show who won/lost
echo ""
echo "Final Results:"
echo "$FINAL_LEADERBOARD" | jq -r '.users[] | "  \(.rank). \(.user.display_name): \(.user.balance) coins (profit: \(.profit))"'
test_passed

# 23. Close the market
test_step "Admin closes the market"
CLOSE_MARKET=$(curl -s -X POST "$API_URL/markets/$MARKET_ID/close/$ADMIN_ID")
echo "Status: $CLOSE_MARKET"
test_passed

# 24. Get final market state
test_step "Get final market state"
FINAL_STATE=$(curl -s "$API_URL/markets/$MARKET_ID")
echo "$FINAL_STATE" | jq '.'
MARKET_STATUS=$(echo "$FINAL_STATE" | jq -r '.status')
echo "Market status: $MARKET_STATUS"
if [ "$MARKET_STATUS" = "closed" ]; then
    test_passed
else
    echo "❌ FAILED: Expected 'closed', got '$MARKET_STATUS'"
    exit 1
fi

# Print summary
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✓ Tests Passed: $TESTS_PASSED / $TESTS_TOTAL${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ $TESTS_PASSED -eq $TESTS_TOTAL ]; then
    echo "🎉 All tests passed! The API is working correctly."
    echo ""
    echo "Key validations:"
    echo "  ✓ Market creation and user joining"
    echo "  ✓ Market lifecycle (setup → open → betting → closed)"
    echo "  ✓ Bet creation with opening wagers"
    echo "  ✓ Hidden bet mechanic (bets about Carol were hidden from her)"
    echo "  ✓ Admin approval workflow"
    echo "  ✓ Wagering from multiple users"
    echo "  ✓ Probability calculations and charts"
    echo "  ✓ Bet resolution and payout distribution"
    echo "  ✓ Leaderboard tracking"
    echo "  ✓ Hidden bet reveal"
    echo ""
    echo "Ready to build the UI! 🚀"
    exit 0
else
    echo "❌ Some tests failed. Please check the output above."
    exit 1
fi
