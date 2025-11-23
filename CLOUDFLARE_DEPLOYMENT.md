# Cloudflare Deployment Reference

This document explains how Cazino can be deployed to Cloudflare Workers while keeping the codebase open-source.

## Architecture

Cazino is designed with a **clean separation between game logic and infrastructure**:

### Open-Source Layer (This Repository)
- ✅ All game logic and business rules
- ✅ Domain models (Market, Bet, User, Wager)
- ✅ Service layer (CazinoService)
- ✅ Database trait abstraction
- ✅ Complete test suite
- ✅ Parimutuel calculations
- ✅ Game rules validation

### Infrastructure Layer (Private Deployment)
- 🔒 Cloud-specific database adapters (D1, Postgres, etc.)
- 🔒 HTTP framework adapters (Axum, worker-rs, etc.)
- 🔒 Domain and routing configuration
- 🔒 Deployment secrets

## Reference Implementation

A reference Cloudflare Workers deployment is maintained privately at:
`github.com/yourusername/y13-platform/workers/cazino`

This implementation demonstrates:
1. **D1 Database Adapter**: Implements the `Database` trait for Cloudflare D1
2. **worker-rs HTTP Layer**: Replaces Axum with Cloudflare's worker-rs
3. **Wrangler Configuration**: Deployment config for edge deployment

## Key Design Decisions

### 1. Database Trait Abstraction

The `Database` trait in `src/db/trait.rs` allows swapping implementations:

```rust
#[async_trait]
pub trait Database: Send + Sync {
    async fn create_market(&self, market: Market) -> DbResult<Market>;
    async fn get_market(&self, id: Uuid) -> DbResult<Market>;
    // ... more methods
}
```

**Implementations**:
- `SqliteDatabase` (this repo) - For local development
- `D1Adapter` (private) - For Cloudflare Workers
- `PostgresDatabase` (community) - For traditional hosting
- `SupabaseDatabase` (future) - For Supabase deployment

### 2. Service Layer

The `CazinoService` struct is generic over any `Database` implementation:

```rust
pub struct CazinoService<D: Database> {
    db: Arc<D>,
}
```

This means the same game logic works with any database backend.

### 3. Domain-Driven Design

All business logic lives in:
- `src/domain/models.rs` - Data structures
- `src/domain/rules.rs` - Game rules validation
- `src/domain/parimutuel.rs` - Betting calculations
- `src/service.rs` - Orchestration

These modules have **zero** infrastructure dependencies.

## Deploying to Cloudflare

### Prerequisites

1. Rust with `wasm32-unknown-unknown` target
2. Wrangler CLI
3. Cloudflare account with Workers enabled

### Steps

1. **Create a Cloudflare Worker project**:
   ```bash
   mkdir cazino-worker
   cd cazino-worker
   ```

2. **Add Cazino as a dependency**:
   ```toml
   # Cargo.toml
   [dependencies]
   cazino = { git = "https://github.com/joabzzz/cazino", branch = "main" }
   worker = "0.6.1"
   ```

3. **Implement D1 Adapter**:
   Create `src/d1_adapter.rs` implementing the `Database` trait for Cloudflare D1.
   
   See reference implementation for details.

4. **Create HTTP Routes**:
   Use `worker-rs` to create HTTP endpoints that call `CazinoService` methods.

5. **Configure Wrangler**:
   ```toml
   # wrangler.toml
   name = "cazino"
   main = "build/worker/shim.mjs"
   
   [[d1_databases]]
   binding = "CAZINO_DB"
   database_name = "cazino"
   ```

6. **Deploy**:
   ```bash
   wrangler deploy
   ```

### What Stays Private

- D1 database IDs
- Domain routing configuration
- Analytics and monitoring setup
- Infrastructure secrets

### What Stays Public

- All game logic
- Database trait interface
- Service layer
- Tests

## Alternative Deployments

This architecture supports many deployment targets:

| Platform | Database | HTTP Framework | Status |
|----------|----------|----------------|--------|
| Cloudflare Workers | D1 | worker-rs | ✅ Reference impl |
| Fly.io | SQLite | Axum | ✅ This repo |
| Railway | Postgres | Axum | 🔄 Community |
| Vercel | Supabase | Axum | 🔄 Future |
| Docker | SQLite/Postgres | Axum | ✅ This repo |

## Benefits

1. **Portability**: Run anywhere, not locked to one cloud
2. **Testing**: Full test suite runs locally with SQLite
3. **Transparency**: Game rules are auditable
4. **Community**: Others can deploy their own instances
5. **Privacy**: Your domain/deployment stays private

## Contributing Infrastructure Adapters

If you create a new database adapter (Postgres, Supabase, etc.), consider:

1. Creating a separate repo: `cazino-postgres-adapter`
2. Implementing the `Database` trait
3. Submitting a PR to add it to this README

This keeps the core game logic clean while expanding deployment options.

## Questions?

- **Can I use this commercially?** Yes, MIT license
- **Can I modify the game rules?** Yes, fork and customize
- **Can I keep my deployment private?** Yes, only deployment config needs to be private
- **Do I need Cloudflare?** No, works with any backend

## Reference

For a complete Cloudflare Workers implementation, see:
- Private reference at `y13-platform/workers/cazino` (contact maintainer)
- Or build your own following the architecture above

---

**License**: MIT
**Game Logic**: 100% open-source
**Deployment Config**: Your choice (public or private)
