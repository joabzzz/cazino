#!/usr/bin/env bash
set -euo pipefail

# Test WebSocket and UI functionality on Cloudflare deployment
#
# Usage:
#   ./scripts/test_cloudflare.sh https://your-app.pages.dev
#
# This script runs Playwright tests against a deployed Cloudflare instance
# to verify that WebSockets and all UI features work correctly in production.

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <cloudflare-url>"
  echo ""
  echo "Examples:"
  echo "  $0 https://cazino.pages.dev"
  echo "  $0 https://your-custom-domain.com"
  exit 1
fi

CLOUDFLARE_URL="$1"

# Remove trailing slash
CLOUDFLARE_URL="${CLOUDFLARE_URL%/}"

echo "🧪 Testing Cazino deployment at: $CLOUDFLARE_URL"
echo ""

# Check if the URL is accessible
echo "1. Checking if deployment is accessible..."
if ! curl -sf "${CLOUDFLARE_URL}" > /dev/null; then
  echo "❌ Error: Cannot reach ${CLOUDFLARE_URL}"
  echo "   Please check that the URL is correct and the deployment is live."
  exit 1
fi
echo "✅ Deployment is accessible"
echo ""

# Check if API health endpoint works
echo "2. Checking API endpoint..."
if curl -sf "${CLOUDFLARE_URL}/api/health" > /dev/null 2>&1 || \
   curl -sf "${CLOUDFLARE_URL}/health" > /dev/null 2>&1; then
  echo "✅ API endpoint is accessible"
else
  echo "⚠️  Warning: API health endpoint not accessible"
  echo "   This may be expected if your API is not yet deployed."
  echo "   Tests may fail if API is required."
fi
echo ""

# Install Playwright browsers if needed
if ! npx playwright --version > /dev/null 2>&1; then
  echo "📦 Installing Playwright..."
  npm install
  npx playwright install chromium
fi

# Run the tests
echo "3. Running Playwright tests against Cloudflare deployment..."
echo ""

export E2E_BASE_URL="${CLOUDFLARE_URL}"

# Run all tests
npx playwright test -c tests/playwright.config.ts

echo ""
echo "✅ Cloudflare deployment tests complete!"
echo ""
echo "To run specific test files:"
echo "  E2E_BASE_URL=${CLOUDFLARE_URL} npx playwright test tests/e2e/cloudflare.spec.ts"
echo "  E2E_BASE_URL=${CLOUDFLARE_URL} npx playwright test tests/e2e/websocket.spec.ts"
echo ""
echo "To run in headed mode (see browser):"
echo "  E2E_BASE_URL=${CLOUDFLARE_URL} npx playwright test --headed"
