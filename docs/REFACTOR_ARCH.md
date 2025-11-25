# Cazino Refactor Architecture Plan

## Executive Summary

This document outlines the architecture for a production-ready, closed-source fork of Cazino. The goal is to create a well-organized, fully-tested, Cloudflare-native prediction market platform.

**Key Decisions:**
- **Backend:** Keep Rust for core logic (domain, parimutuel engine) - compile to WASM for Workers
- **WebSocket Server:** TypeScript (Cloudflare Durable Objects) - required for stateful connections
- **UI:** Migrate to React/Preact with TypeScript - enables proper testing and component architecture
- **Database:** Cloudflare D1 (SQLite-compatible) - already partially implemented
- **Testing:** Vitest for unit tests, Playwright for E2E, Rust tests for domain logic

---

## Current State Analysis

### What Works Well
1. **Domain logic** (`src/domain/`) - Clean parimutuel calculations, visibility rules
2. **Database trait abstraction** - Can swap SQLite ↔ D1
3. **Service layer pattern** - Business logic centralized
4. **Basic E2E tests** - Playwright infrastructure exists

### Critical Issues
1. **No authentication/authorization** - Anyone can impersonate any user
2. **WebSocket broadcasts globally** - No per-market subscription
3. **UI is vanilla JS** - Hard to test, no component structure
4. **Worker code has 15 service clones** - Awkward routing pattern
5. **Migrations split** - SQLite inline vs D1 file-based
6. **No proper error codes** - Everything is HTTP 500

---

## Target Architecture

```
cazino/
├── packages/
│   ├── core/                    # Shared Rust domain logic → WASM
│   │   ├── src/
│   │   │   ├── domain/          # Models, rules, parimutuel
│   │   │   ├── service/         # Business logic orchestration
│   │   │   └── lib.rs           # WASM bindings
│   │   ├── Cargo.toml
│   │   └── tests/
│   │
│   ├── api/                     # Cloudflare Worker (HTTP API)
│   │   ├── src/
│   │   │   ├── routes/          # Route handlers by resource
│   │   │   ├── middleware/      # Auth, CORS, error handling
│   │   │   ├── db/              # D1 database layer
│   │   │   └── index.ts
│   │   ├── wrangler.toml
│   │   └── tests/
│   │
│   ├── realtime/                # Cloudflare Durable Objects (WebSocket)
│   │   ├── src/
│   │   │   ├── MarketRoom.ts    # Per-market WebSocket handler
│   │   │   ├── messages.ts      # Message types
│   │   │   └── index.ts
│   │   ├── wrangler.toml
│   │   └── tests/
│   │
│   ├── ui/                      # React/Preact SPA
│   │   ├── src/
│   │   │   ├── components/      # Reusable UI components
│   │   │   ├── screens/         # Page-level components
│   │   │   ├── hooks/           # Custom React hooks
│   │   │   ├── api/             # API client
│   │   │   ├── store/           # State management
│   │   │   └── App.tsx
│   │   ├── vite.config.ts
│   │   └── tests/
│   │
│   └── shared/                  # Shared TypeScript types
│       ├── src/
│       │   ├── api.ts           # API request/response types
│       │   ├── domain.ts        # Domain model types
│       │   └── websocket.ts     # WebSocket message types
│       └── package.json
│
├── e2e/                         # End-to-end tests
│   ├── tests/
│   │   ├── market-flow.spec.ts
│   │   ├── betting.spec.ts
│   │   ├── websocket.spec.ts
│   │   └── admin.spec.ts
│   └── playwright.config.ts
│
├── migrations/                  # D1 migrations (single source of truth)
│   ├── 0001_initial_schema.sql
│   ├── 0002_add_challenges.sql
│   └── ...
│
├── scripts/                     # Dev/deploy scripts
│   ├── dev.sh                   # Start all services locally
│   ├── deploy.sh                # Deploy to Cloudflare
│   └── migrate.sh               # Run D1 migrations
│
├── turbo.json                   # Turborepo config
├── package.json                 # Root workspace config
└── pnpm-workspace.yaml
```

---

## Package Details

### 1. `packages/core` (Rust → WASM)

**Purpose:** Portable domain logic that runs in both Workers and tests

**Keep from current:**
- `domain/models.rs` - Market, User, Bet, Wager, Side, etc.
- `domain/parimutuel.rs` - Probability calculations, payout logic
- `domain/rules.rs` - Validation rules

**Changes:**
- Remove all Axum/HTTP dependencies
- Add `wasm-bindgen` exports for TypeScript consumption
- Service layer becomes pure functions (no DB access)

```rust
// packages/core/src/lib.rs
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn calculate_probability(yes_pool: f64, no_pool: f64) -> f64 {
    domain::parimutuel::calculate_probability(yes_pool, no_pool)
}

#[wasm_bindgen]
pub fn validate_wager(
    market_status: &str,
    bet_status: &str,
    user_balance: f64,
    wager_amount: f64,
    is_subject: bool,
) -> Result<(), String> {
    // ...
}
```

**Testing:**
- Rust unit tests for all domain logic
- Property-based tests for parimutuel math

---

### 2. `packages/api` (TypeScript Worker)

**Purpose:** HTTP API running on Cloudflare Workers

**Structure:**
```
api/src/
├── index.ts              # Worker entry point
├── router.ts             # Hono or itty-router setup
├── routes/
│   ├── markets.ts        # /api/markets/*
│   ├── bets.ts           # /api/bets/*
│   ├── users.ts          # /api/users/*
│   └── devices.ts        # /api/devices/*
├── middleware/
│   ├── auth.ts           # Device token validation
│   ├── cors.ts           # CORS headers
│   └── errors.ts         # Error handling
├── db/
│   ├── client.ts         # D1 client wrapper
│   ├── markets.ts        # Market queries
│   ├── bets.ts           # Bet queries
│   ├── users.ts          # User queries
│   └── wagers.ts         # Wager queries
└── services/
    ├── market.service.ts # Market business logic
    ├── bet.service.ts    # Bet business logic
    └── wager.service.ts  # Wager business logic
```

**Key Changes from Current:**

1. **Proper Authentication:**
```typescript
// middleware/auth.ts
export async function authMiddleware(c: Context, next: Next) {
  const deviceId = c.req.header('X-Device-ID');
  const signature = c.req.header('X-Device-Signature');

  if (!deviceId || !verifySignature(deviceId, signature)) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  c.set('deviceId', deviceId);
  await next();
}
```

2. **Typed Error Responses:**
```typescript
// middleware/errors.ts
export class ApiError extends Error {
  constructor(
    public code: string,
    public message: string,
    public status: number = 400
  ) {
    super(message);
  }
}

// Usage
throw new ApiError('INSUFFICIENT_BALANCE', 'Not enough coins', 400);
throw new ApiError('NOT_ADMIN', 'Admin required', 403);
throw new ApiError('MARKET_CLOSED', 'Market is not open', 400);
```

3. **Router Pattern (using Hono):**
```typescript
// index.ts
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { markets } from './routes/markets';
import { bets } from './routes/bets';

const app = new Hono<{ Bindings: Env }>();

app.use('*', cors());
app.use('/api/*', authMiddleware);

app.route('/api/markets', markets);
app.route('/api/bets', bets);

export default app;
```

**Testing:**
- Vitest with miniflare for D1 mocking
- Unit tests for each service
- Integration tests for routes

---

### 3. `packages/realtime` (Durable Objects)

**Purpose:** Per-market WebSocket rooms with state

**Why TypeScript is Required:**
- Durable Objects are a Cloudflare-specific feature
- No Rust support for DO WebSocket hibernation API
- Enables proper per-market subscription

**Structure:**
```
realtime/src/
├── index.ts              # Worker entry, DO exports
├── MarketRoom.ts         # Durable Object class
├── messages.ts           # Message type definitions
└── broadcast.ts          # Broadcast helpers
```

**MarketRoom Implementation:**
```typescript
// MarketRoom.ts
import { DurableObject } from 'cloudflare:workers';

export class MarketRoom extends DurableObject {
  private connections: Map<string, WebSocket> = new Map();

  async fetch(request: Request): Promise<Response> {
    if (request.headers.get('Upgrade') === 'websocket') {
      return this.handleWebSocket(request);
    }

    // Handle broadcast requests from API worker
    if (request.method === 'POST') {
      const message = await request.json();
      this.broadcast(message);
      return new Response('OK');
    }

    return new Response('Not found', { status: 404 });
  }

  private handleWebSocket(request: Request): Response {
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);

    const userId = new URL(request.url).searchParams.get('userId');
    this.connections.set(userId, server);

    server.accept();
    server.addEventListener('close', () => {
      this.connections.delete(userId);
    });

    return new Response(null, { status: 101, webSocket: client });
  }

  private broadcast(message: WsMessage) {
    const payload = JSON.stringify(message);
    for (const [userId, ws] of this.connections) {
      // Optional: filter by user for targeted messages
      ws.send(payload);
    }
  }
}
```

**API Integration:**
```typescript
// In api/src/services/bet.service.ts
async function notifyBetCreated(env: Env, marketId: string, bet: Bet) {
  const roomId = env.MARKET_ROOMS.idFromName(marketId);
  const room = env.MARKET_ROOMS.get(roomId);

  await room.fetch('https://room/broadcast', {
    method: 'POST',
    body: JSON.stringify({
      type: 'BetCreated',
      payload: bet,
    }),
  });
}
```

**Testing:**
- Miniflare for Durable Object simulation
- WebSocket connection tests

---

### 4. `packages/ui` (React/Preact)

**Purpose:** Modern, testable UI with component architecture

**Tech Stack:**
- **Framework:** Preact (smaller bundle, React-compatible)
- **Styling:** Tailwind CSS or vanilla CSS (keep current styles)
- **State:** Zustand (simple, no boilerplate)
- **Routing:** preact-router (lightweight)
- **Build:** Vite

**Structure:**
```
ui/src/
├── App.tsx
├── main.tsx
├── components/
│   ├── Button.tsx
│   ├── Card.tsx
│   ├── Modal.tsx
│   ├── BetCard.tsx
│   ├── ProbabilityChart.tsx
│   ├── Leaderboard.tsx
│   └── ...
├── screens/
│   ├── Landing.tsx
│   ├── CreateMarket.tsx
│   ├── JoinMarket.tsx
│   ├── Lobby.tsx
│   ├── MarketView.tsx
│   ├── BetDetail.tsx
│   └── Reveal.tsx
├── hooks/
│   ├── useApi.ts           # API client hook
│   ├── useWebSocket.ts     # WebSocket connection hook
│   ├── useDevice.ts        # Device ID management
│   └── useMarket.ts        # Market state hook
├── store/
│   ├── index.ts
│   ├── marketStore.ts
│   └── userStore.ts
├── api/
│   ├── client.ts           # Fetch wrapper
│   ├── markets.ts          # Market API calls
│   ├── bets.ts             # Bet API calls
│   └── types.ts            # Generated from shared
└── utils/
    ├── device.ts           # Device fingerprinting
    └── format.ts           # Number/date formatting
```

**Key Component Example:**
```tsx
// screens/MarketView.tsx
import { useParams } from 'preact-router';
import { useMarket } from '../hooks/useMarket';
import { useWebSocket } from '../hooks/useWebSocket';
import { BetCard } from '../components/BetCard';

export function MarketView() {
  const { marketId } = useParams();
  const { market, bets, loading } = useMarket(marketId);

  useWebSocket(marketId, {
    onBetCreated: (bet) => { /* update state */ },
    onWagerPlaced: (wager) => { /* update state */ },
  });

  if (loading) return <Spinner />;

  return (
    <div class="market-view">
      <h1>{market.name}</h1>
      <div class="bets-grid">
        {bets.map(bet => <BetCard key={bet.id} bet={bet} />)}
      </div>
    </div>
  );
}
```

**Testing:**
- Vitest + @testing-library/preact for unit tests
- Component tests with mock API responses
- Visual regression tests (optional)

---

### 5. `packages/shared` (TypeScript Types)

**Purpose:** Single source of truth for API contracts

```typescript
// shared/src/api.ts
export interface CreateMarketRequest {
  name: string;
  startingBalance: number;
  displayName: string;
  avatar: string;
}

export interface CreateMarketResponse {
  market: Market;
  user: User;
  inviteCode: string;
}

export interface ApiError {
  code: string;
  message: string;
}

// shared/src/domain.ts
export interface Market {
  id: string;
  name: string;
  status: 'draft' | 'open' | 'closed' | 'resolved';
  createdBy: string;
  inviteCode: string;
  startingBalance: number;
  createdAt: string;
}

export interface Bet {
  id: string;
  marketId: string;
  subjectUserId: string;
  createdBy: string;
  description: string;
  status: 'pending' | 'active' | 'resolved_yes' | 'resolved_no';
  yesPool: number;
  noPool: number;
  hideFromSubject: boolean;
}

// shared/src/websocket.ts
export type WsMessage =
  | { type: 'BetCreated'; payload: Bet }
  | { type: 'WagerPlaced'; payload: Wager }
  | { type: 'MarketStatusChanged'; payload: { status: Market['status'] } }
  | { type: 'UserJoined'; payload: User };
```

---

## Database Schema

**Migrate to single migration system (D1):**

```sql
-- migrations/0001_initial_schema.sql
CREATE TABLE markets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    created_by TEXT NOT NULL,
    invite_code TEXT UNIQUE NOT NULL,
    starting_balance INTEGER NOT NULL DEFAULT 1000,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    opens_at TEXT,
    closes_at TEXT
);

CREATE TABLE users (
    id TEXT PRIMARY KEY,
    market_id TEXT NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar TEXT NOT NULL,
    balance INTEGER NOT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    joined_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(market_id, device_id)
);

CREATE TABLE bets (
    id TEXT PRIMARY KEY,
    market_id TEXT NOT NULL REFERENCES markets(id) ON DELETE CASCADE,
    subject_user_id TEXT NOT NULL REFERENCES users(id),
    created_by TEXT NOT NULL REFERENCES users(id),
    description TEXT NOT NULL,
    initial_odds TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    yes_pool INTEGER NOT NULL DEFAULT 0,
    no_pool INTEGER NOT NULL DEFAULT 0,
    hide_from_subject INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved_at TEXT
);

CREATE TABLE wagers (
    id TEXT PRIMARY KEY,
    bet_id TEXT NOT NULL REFERENCES bets(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id),
    side TEXT NOT NULL,
    amount INTEGER NOT NULL,
    placed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    yes_pool_after INTEGER NOT NULL,
    no_pool_after INTEGER NOT NULL,
    probability_after REAL NOT NULL
);

-- Indexes
CREATE INDEX idx_users_market ON users(market_id);
CREATE INDEX idx_users_device ON users(market_id, device_id);
CREATE INDEX idx_bets_market ON bets(market_id);
CREATE INDEX idx_bets_status ON bets(market_id, status);
CREATE INDEX idx_bets_subject ON bets(subject_user_id);
CREATE INDEX idx_wagers_bet ON wagers(bet_id);
CREATE INDEX idx_wagers_user ON wagers(user_id);
```

---

## Testing Strategy

### Unit Tests

| Package | Framework | Coverage Target |
|---------|-----------|-----------------|
| core | Rust tests | 90%+ |
| api | Vitest | 80%+ |
| realtime | Vitest | 80%+ |
| ui | Vitest + Testing Library | 70%+ |

### Integration Tests

```typescript
// api/tests/integration/markets.test.ts
import { describe, it, expect, beforeAll } from 'vitest';
import { createTestEnv } from '../helpers';

describe('Markets API', () => {
  let env: TestEnv;

  beforeAll(async () => {
    env = await createTestEnv();
  });

  it('creates a market with admin user', async () => {
    const response = await env.fetch('/api/markets', {
      method: 'POST',
      body: JSON.stringify({
        name: 'Test Market',
        startingBalance: 1000,
        displayName: 'Admin',
        avatar: '🎰',
      }),
    });

    expect(response.status).toBe(200);
    const data = await response.json();
    expect(data.market.name).toBe('Test Market');
    expect(data.user.isAdmin).toBe(true);
    expect(data.inviteCode).toMatch(/^[A-Z0-9]{6}$/);
  });
});
```

### E2E Tests (Playwright)

```typescript
// e2e/tests/market-flow.spec.ts
import { test, expect } from '@playwright/test';

test('complete market flow', async ({ page, context }) => {
  // Admin creates market
  await page.goto('/');
  await page.click('text=Create Market');
  await page.fill('[name=name]', 'Family Game Night');
  await page.fill('[name=displayName]', 'Admin');
  await page.click('text=Create');

  const inviteCode = await page.textContent('.invite-code');

  // Player joins in another tab
  const playerPage = await context.newPage();
  await playerPage.goto('/');
  await playerPage.click('text=Join Market');
  await playerPage.fill('[name=code]', inviteCode);
  await playerPage.fill('[name=displayName]', 'Player1');
  await playerPage.click('text=Join');

  // Verify player appears in admin's lobby
  await expect(page.locator('text=Player1')).toBeVisible();
});
```

---

## Migration Plan

### Phase 1: Setup Monorepo
1. Initialize pnpm workspace with Turborepo
2. Create package structure
3. Set up shared types package
4. Configure build tooling

### Phase 2: Extract Core Logic
1. Copy domain logic to `packages/core`
2. Remove HTTP dependencies from Rust code
3. Add WASM bindings
4. Write comprehensive Rust tests

### Phase 3: Build API Package
1. Create Hono-based API worker
2. Implement D1 database layer
3. Add authentication middleware
4. Migrate all routes with proper error handling
5. Write integration tests

### Phase 4: Build Realtime Package
1. Create Durable Object for MarketRoom
2. Implement WebSocket handlers
3. Connect API → DO for broadcasts
4. Test WebSocket flows

### Phase 5: Build UI Package
1. Set up Preact with Vite
2. Create component library
3. Migrate screens from vanilla JS
4. Add state management
5. Connect to API and WebSocket
6. Write component tests

### Phase 6: E2E & Polish
1. Write comprehensive E2E tests
2. Performance optimization
3. Error handling polish
4. Documentation

---

## Cloudflare Configuration

### API Worker (`packages/api/wrangler.toml`)
```toml
name = "cazino-api"
main = "src/index.ts"
compatibility_date = "2024-11-01"

[[d1_databases]]
binding = "DB"
database_name = "cazino"
database_id = "xxx"

[durable_objects]
bindings = [
  { name = "MARKET_ROOMS", class_name = "MarketRoom", script_name = "cazino-realtime" }
]
```

### Realtime Worker (`packages/realtime/wrangler.toml`)
```toml
name = "cazino-realtime"
main = "src/index.ts"
compatibility_date = "2024-11-01"

[durable_objects]
bindings = [
  { name = "MARKET_ROOMS", class_name = "MarketRoom" }
]

[[migrations]]
tag = "v1"
new_classes = ["MarketRoom"]
```

### Pages (`packages/ui/wrangler.toml`)
```toml
name = "cazino"
pages_build_output_dir = "dist"

[env.production]
vars = { PUBLIC_API_URL = "https://api.cazino.app" }

[env.preview]
vars = { PUBLIC_API_URL = "https://api-preview.cazino.app" }
```

---

## Security Improvements

### 1. Device Authentication
```typescript
// Device generates keypair on first use
const keypair = await crypto.subtle.generateKey(
  { name: 'ECDSA', namedCurve: 'P-256' },
  true,
  ['sign', 'verify']
);

// Store public key with device ID
await registerDevice(deviceId, publicKey);

// Sign requests
const signature = await crypto.subtle.sign(
  { name: 'ECDSA', hash: 'SHA-256' },
  privateKey,
  new TextEncoder().encode(requestBody)
);
```

### 2. Rate Limiting
```typescript
// Use Cloudflare's built-in rate limiting
export default {
  async fetch(request, env) {
    const { success } = await env.RATE_LIMITER.limit({ key: getDeviceId(request) });
    if (!success) {
      return new Response('Rate limited', { status: 429 });
    }
    // ...
  }
};
```

### 3. Input Validation
```typescript
import { z } from 'zod';

const CreateMarketSchema = z.object({
  name: z.string().min(1).max(100),
  startingBalance: z.number().int().min(100).max(10000),
  displayName: z.string().min(1).max(50),
  avatar: z.string().emoji(),
});
```

---

## Open Questions

1. **Rust vs Full TypeScript?**
   - Rust WASM adds complexity but provides type-safe domain logic
   - Could implement parimutuel in TS if team prefers
   - Recommendation: Keep Rust for now, can migrate later if needed

2. **State Management?**
   - Zustand is simple and works well with Preact
   - Could use Jotai for more atomic state
   - Or even just React Context for this scale

3. **PWA Requirements?**
   - Current app has PWA support
   - Preact + Vite PWA plugin can replicate
   - Need to decide on offline capabilities

4. **Challenge System?**
   - Infrastructure exists but incomplete
   - Include in v1 or defer?

---

## Appendix: File Mapping (Current → New)

| Current | New Location |
|---------|--------------|
| `src/domain/models.rs` | `packages/core/src/domain/models.rs` |
| `src/domain/parimutuel.rs` | `packages/core/src/domain/parimutuel.rs` |
| `src/domain/rules.rs` | `packages/core/src/domain/rules.rs` |
| `src/service.rs` | Split into `packages/api/src/services/*.ts` |
| `src/api/routes.rs` | Split into `packages/api/src/routes/*.ts` |
| `src/api/websocket.rs` | `packages/realtime/src/MarketRoom.ts` |
| `src/db/sqlite.rs` | Removed (D1 only) |
| `src/db/d1.rs` | `packages/api/src/db/*.ts` |
| `worker/src/lib.rs` | `packages/api/src/index.ts` |
| `worker/src/room.rs` | `packages/realtime/src/MarketRoom.ts` |
| `ui/app.js` | Split into `packages/ui/src/screens/*.tsx` |
| `ui/style.css` | `packages/ui/src/styles/` or Tailwind |
| `tests/e2e/*.spec.ts` | `e2e/tests/*.spec.ts` |
