-- Initial schema for Cazino
-- Matches the SQLite implementation exactly

CREATE TABLE IF NOT EXISTS markets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_by TEXT NOT NULL,
    opens_at TEXT NOT NULL,
    closes_at TEXT NOT NULL,
    starting_balance INTEGER NOT NULL,
    invite_code TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    market_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar TEXT NOT NULL,
    balance INTEGER NOT NULL,
    is_admin INTEGER NOT NULL,
    joined_at TEXT NOT NULL,
    FOREIGN KEY (market_id) REFERENCES markets(id),
    UNIQUE(market_id, device_id)
);

CREATE TABLE IF NOT EXISTS bets (
    id TEXT PRIMARY KEY,
    market_id TEXT NOT NULL,
    subject_user_id TEXT NOT NULL,
    created_by TEXT NOT NULL,
    description TEXT NOT NULL,
    initial_odds TEXT NOT NULL,
    status TEXT NOT NULL,
    yes_pool INTEGER NOT NULL,
    no_pool INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (market_id) REFERENCES markets(id),
    FOREIGN KEY (subject_user_id) REFERENCES users(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS wagers (
    id TEXT PRIMARY KEY,
    bet_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    side TEXT NOT NULL,
    amount INTEGER NOT NULL,
    placed_at TEXT NOT NULL,
    yes_pool_after INTEGER NOT NULL,
    no_pool_after INTEGER NOT NULL,
    probability_after REAL NOT NULL,
    FOREIGN KEY (bet_id) REFERENCES bets(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_users_market ON users(market_id);
CREATE INDEX IF NOT EXISTS idx_users_device ON users(market_id, device_id);
CREATE INDEX IF NOT EXISTS idx_bets_market ON bets(market_id);
CREATE INDEX IF NOT EXISTS idx_bets_status ON bets(status);
CREATE INDEX IF NOT EXISTS idx_bets_subject ON bets(subject_user_id);
CREATE INDEX IF NOT EXISTS idx_wagers_bet ON wagers(bet_id);
CREATE INDEX IF NOT EXISTS idx_wagers_user ON wagers(user_id);
