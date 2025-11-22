# Cazino

Private betting markets for families and friends. 

## Overview

Cazino is a social betting game where players create prediction markets about real-life events and place bets about each other. The core mechanic: players cannot see bets about themselves until they're resolved. This is an unregulated betting market and cheating is encouraged!

**Key Features:**
- Hidden bet mechanic (bets about you are invisible until resolved)
- Parimutuel betting pools with real-time probability calculations
- No-password authentication via invite codes (Jackbox-style)
- Real-time updates via WebSocket
- SQLite for local development, designed to swap to Supabase for production

## Quick Start

### Prerequisites
- Rust 1.70 or later ([install here](https://rustup.rs))
- SQLite (usually pre-installed on macOS/Linux)

### Installation

```bash
git clone <your-repo-url>
cd cazino
cargo build --release
cargo run --release -- serve
```

Server starts on `http://localhost:3000`

## Testing

### Run Unit & Integration Tests
```bash
cargo test
```

15 tests covering game lifecycle, parimutuel calculations, hidden bets, and payouts.

### Run API Tests (End-to-End)
```bash
./tests/test_api.sh
```

24 tests simulating 3 users through a complete game, covering:
- Market creation and user joining
- Hidden bet mechanic
- Bet approval workflow
- Multi-user wagering
- Real-time probability calculations
- Bet resolution and payouts
- Leaderboard tracking

### Test WebSocket Real-Time Updates
```bash
brew install websocat  # if not installed
./tests/test_websocket.sh
```

## API Overview

### Key Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/api/markets` | POST | Create a market |
| `/api/markets/:invite_code/join` | POST | Join a market |
| `/api/markets/:market_id/open/:admin_id` | POST | Open market for betting |
| `/api/markets/:market_id/bets/:creator_id/create` | POST | Create a bet |
| `/api/bets/:bet_id/approve/:admin_id` | POST | Approve a bet |
| `/api/bets/:bet_id/wager/:user_id` | POST | Place a wager |
| `/api/bets/:bet_id/resolve/:admin_id` | POST | Resolve a bet |
| `/api/markets/:market_id/leaderboard` | GET | Get user rankings |
| `/api/users/:user_id/reveal` | GET | Reveal hidden bets |

See [API.md](./API.md) for complete documentation.

### WebSocket

Connect to `ws://localhost:3000/ws` for real-time updates.

Subscribe to a market:
```json
{"type": "subscribe", "market_id": "uuid-here"}
```

Receive updates:
```json
{"type": "wager_placed", "bet_id": "...", "new_probability": 0.67}
{"type": "bet_resolved", "bet_id": "...", "outcome": "YES"}
```

## Architecture

```
cazino/
├── src/
│   ├── domain/          # Pure business logic
│   │   ├── models.rs    # Core data structures
│   │   ├── parimutuel.rs # Betting math
│   │   └── rules.rs     # Game rules & validation
│   ├── db/              # Database layer
│   │   ├── trait.rs     # Database abstraction
│   │   └── sqlite.rs    # SQLite implementation
│   ├── service.rs       # Business logic orchestration
│   ├── api/             # HTTP + WebSocket API
│   └── cli/             # Interactive CLI
└── tests/               # Test scripts and integration tests
```

### Design Principles

- **Clean Architecture**: Domain logic is pure and database-agnostic
- **Trait Abstraction**: Easy to swap SQLite for Supabase
- **Domain-Layer Filtering**: Hidden bet logic is portable across database implementations
- **Parimutuel Math**: Self-balancing pools with no house edge
- **Real-Time First**: WebSocket broadcasts for instant updates

## Game Rules

### Market Lifecycle
1. **Draft** - Collecting players and bet proposals
2. **Open** - Active betting period
3. **Closed** - Resolution period (admin resolves bets)
4. **Resolved** - Final results

### Betting Rules
- Players can bet on anyone except themselves
- All bets require admin approval before going live
- Minimum bet: 1 coin
- Cannot wager more than current balance
- Cannot bet on bets about yourself
- Wagers cannot be canceled once placed

### Hidden Bet Mechanic
- If a bet is about you and not yet resolved, it appears as `[Hidden - about you]`
- Hidden bets redact description and subject information
- Use `/api/users/:user_id/reveal` to view bets about yourself
- Once resolved, bets are automatically revealed to all players

### Parimutuel Payouts
Winners split the losing pool proportionally to their stake.

**Example:**
- YES pool: 300 coins (3 bettors)
- NO pool: 150 coins (2 bettors)
- Outcome: YES wins
- Total pot: 450 coins distributed to YES bettors
- If you wagered 100 on YES: (100/300) × 450 = 150 coins
- Net profit: 50 coins

## Usage Modes

### API Server Mode
```bash
cargo run -- serve
```

Best for building a UI or testing via HTTP/WebSocket.

### Interactive CLI Mode
```bash
cargo run -- cli
```

Commands: `create`, `join`, `bet`, `wager`, `reveal`, `help`

## Configuration

### Database
SQLite database auto-created as `cazino.db` in the current directory. The trait-based abstraction allows easy migration to Supabase.

### Logging
```bash
# Default (info level)
cargo run -- serve

# Debug level
RUST_LOG=debug cargo run -- serve

# Warnings only
RUST_LOG=warn cargo run -- serve
```

See [LOGGING.md](./LOGGING.md) for details.

## API Data Formats

### Enum Casing
- **Side enum**: `"YES"`, `"NO"` (uppercase)
- **BetStatus**: `"pending"`, `"active"`, `"resolvedyes"`, `"resolvedno"` (lowercase)
- **MarketStatus**: `"draft"`, `"open"`, `"closed"`, `"resolved"` (lowercase)

### Odds Format
Use ratio format: `"1:1"`, `"2:1"`, `"3:1"` (not percentages like `"50-50"`)

## Development Status

### Completed (v1.0)
- Core betting logic
- Hidden bet mechanic
- Parimutuel calculations
- HTTP + WebSocket API
- Comprehensive test coverage
- Production-ready logging

### Roadmap
- Web UI (React/Svelte/Solid)
- Supabase integration
- User avatars and themes
- Bet templates
- Historical statistics
- Multi-market support

## Documentation

- [API.md](./API.md) - Complete API reference
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Deployment guide
- [LOGGING.md](./LOGGING.md) - Logging documentation
- [TEST_RESULTS.md](./TEST_RESULTS.md) - Test coverage details

## Technology Stack

- **Rust** - Systems programming language
- **Axum** - Modern async web framework
- **SQLx** - Compile-time SQL verification
- **Tokio** - Async runtime
- **Tower** - Middleware ecosystem

## License

MIT License - see LICENSE file for details.
