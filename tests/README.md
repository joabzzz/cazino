# Cazino Tests

## Test Structure

```
tests/
├── integration_tests.rs    # Rust service tests (business logic)
├── playwright.config.ts    # Playwright E2E config
└── e2e/
    ├── full-stack.spec.ts  # Complete user flow test
    ├── websocket.spec.ts   # WebSocket real-time tests
    └── cloudflare.spec.ts  # Cloudflare deployment tests
```

## Running Tests

### Rust Integration Tests

Tests the core betting service logic (markets, bets, wagers, payouts).

```bash
# Run all Rust tests
cargo test

# Run with SQLite feature (for integration tests)
cargo test --features sqlite
```

### Playwright E2E Tests

Tests the full UI + API + WebSocket stack.

```bash
# Install dependencies (first time)
npm install
npx playwright install

# Run all E2E tests (starts server automatically)
npm run test:ui

# Run in headed mode (see the browser)
npm run test:ui:headed

# Run specific test file
npx playwright test tests/e2e/full-stack.spec.ts
npx playwright test tests/e2e/websocket.spec.ts
```

### Test Against Cloudflare Deployment

```bash
# Test a deployed instance
E2E_BASE_URL=https://your-app.pages.dev npm run test:cloudflare

# Or use the helper script
./scripts/test_cloudflare.sh https://your-app.pages.dev
```

## Test Descriptions

### `integration_tests.rs`
- Market lifecycle (create, open, close)
- User joining and returning
- Bet creation and approval
- Hidden bet mechanics
- Wager placement and validation
- Parimutuel payout calculations
- Admin-only actions

### `full-stack.spec.ts`
End-to-end happy path:
1. Admin creates market
2. Users join via invite code
3. Admin opens market
4. Bet creation about a user
5. Wager placement
6. Balance and leaderboard updates

### `websocket.spec.ts`
Real-time functionality:
- WebSocket connection on market join
- `user_joined` broadcast
- `market_opened` broadcast
- `bet_created` broadcast
- `wager_placed` broadcast
- Automatic reconnection
- Multi-client broadcasts

### `cloudflare.spec.ts`
Production deployment verification:
- Health endpoints
- API routing
- WebSocket through Cloudflare proxy
- WSS on HTTPS
- Static asset loading
- CORS headers

## Debugging

```bash
# Run with visible browser
npx playwright test --headed

# Debug mode (step through)
PWDEBUG=1 npx playwright test

# View test report after failure
npx playwright show-report

# Check server logs
RUST_LOG=debug cargo run -- serve
```

## Environment Variables

- `E2E_BASE_URL` - Target URL (default: `http://localhost:3333`)
- `RUST_LOG` - Rust logging level (`debug`, `info`, etc.)
