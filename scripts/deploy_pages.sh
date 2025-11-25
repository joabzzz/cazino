#!/usr/bin/env bash
# Deploy Cazino UI to Cloudflare Pages (cazino.y13.io)
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "${ROOT_DIR}"

# Check for wrangler
if ! command -v npx &> /dev/null; then
    echo "Error: npx not found. Please install Node.js and npm" >&2
    exit 1
fi

echo "🎰 Deploying Cazino to Cloudflare Pages"
echo "========================================"
echo ""

echo "📦 Building UI..."
./scripts/build_ui.sh

echo ""
echo "🚀 Deploying to cazino.y13.io..."
npx wrangler pages deploy dist --project-name=cazino

echo ""
echo "✅ Deployment complete!"
echo ""
echo "📋 Post-deployment checklist:"
echo "  1. Verify custom domain 'cazino.y13.io' is configured in Pages dashboard"
echo "  2. Ensure /api and /ws are routed to your API server (Tunnel or Transform Rules)"
echo "  3. Test: https://cazino.y13.io"
echo ""
