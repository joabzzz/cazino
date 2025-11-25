# Cazino Development Commands

# Kill all running cazino server processes
kill-servers:
    @echo "🔍 Looking for running cazino servers..."
    @pkill -f "cazino serve" && echo "✅ Killed all cazino servers" || echo "ℹ️  No cazino servers running"

# Clean database and restart fresh
clean-db:
    @echo "🗑️  Cleaning database..."
    @rm -f cazino.db cazino-e2e.db
    @echo "✅ Database cleaned"

# Setup TEST99 test room (requires server to be running)
test-room:
    @./scripts/setup_test_room.sh

# Kill servers, clean DB, and start fresh
fresh: kill-servers clean-db
    @echo "🚀 Starting fresh server..."
    cargo run --release

# Development mode - kill old servers and start in dev mode
dev: kill-servers clean-db
    @echo "🛠️  Starting development server..."
    cargo run

# Run tests
test:
    cargo test

# Build release binary
build:
    cargo build --release

# Check code
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Lint code
lint:
    cargo clippy
