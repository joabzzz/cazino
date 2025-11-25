/// SQLite implementation of the Database trait
use crate::db::r#trait::{Database, DbError, DbResult};
use crate::domain::models::{Bet, BetStatus, BetView, Market, MarketStatus, Side, User, Wager};
use async_trait::async_trait;
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
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

            CREATE INDEX IF NOT EXISTS idx_users_market ON users(market_id);
            CREATE INDEX IF NOT EXISTS idx_users_device ON users(market_id, device_id);
            CREATE INDEX IF NOT EXISTS idx_bets_market ON bets(market_id);
            CREATE INDEX IF NOT EXISTS idx_bets_status ON bets(status);
            CREATE INDEX IF NOT EXISTS idx_bets_subject ON bets(subject_user_id);
            CREATE INDEX IF NOT EXISTS idx_wagers_bet ON wagers(bet_id);
            CREATE INDEX IF NOT EXISTS idx_wagers_user ON wagers(user_id);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// Helper functions for serialization
fn serialize_market_status(status: MarketStatus) -> String {
    match status {
        MarketStatus::Draft => "draft".to_string(),
        MarketStatus::Open => "open".to_string(),
        MarketStatus::Closed => "closed".to_string(),
        MarketStatus::Resolved => "resolved".to_string(),
    }
}

fn deserialize_market_status(s: &str) -> MarketStatus {
    match s {
        "draft" => MarketStatus::Draft,
        "open" => MarketStatus::Open,
        "closed" => MarketStatus::Closed,
        "resolved" => MarketStatus::Resolved,
        _ => MarketStatus::Draft,
    }
}

fn serialize_bet_status(status: BetStatus) -> String {
    match status {
        BetStatus::Pending => "pending".to_string(),
        BetStatus::Active => "active".to_string(),
        BetStatus::ResolvedYes => "resolved_yes".to_string(),
        BetStatus::ResolvedNo => "resolved_no".to_string(),
        BetStatus::Challenged => "challenged".to_string(),
    }
}

fn deserialize_bet_status(s: &str) -> BetStatus {
    match s {
        "pending" => BetStatus::Pending,
        "active" => BetStatus::Active,
        "resolved_yes" => BetStatus::ResolvedYes,
        "resolved_no" => BetStatus::ResolvedNo,
        "challenged" => BetStatus::Challenged,
        _ => BetStatus::Pending,
    }
}

fn serialize_side(side: Side) -> String {
    match side {
        Side::Yes => "YES".to_string(),
        Side::No => "NO".to_string(),
    }
}

fn deserialize_side(s: &str) -> Side {
    match s {
        "YES" => Side::Yes,
        "NO" => Side::No,
        _ => Side::Yes,
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn create_market(&self, market: Market) -> DbResult<Market> {
        sqlx::query(
            r#"
            INSERT INTO markets (id, name, status, created_by, opens_at, closes_at, starting_balance, invite_code, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(market.id.to_string())
        .bind(&market.name)
        .bind(serialize_market_status(market.status))
        .bind(market.created_by.to_string())
        .bind(market.opens_at.to_rfc3339())
        .bind(market.closes_at.to_rfc3339())
        .bind(market.starting_balance)
        .bind(&market.invite_code)
        .bind(market.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(market)
    }

    async fn get_market(&self, id: Uuid) -> DbResult<Market> {
        let row = sqlx::query("SELECT * FROM markets WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| DbError::NotFound("Market not found".to_string()))?;

        Ok(Market {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            name: row.get("name"),
            status: deserialize_market_status(row.get("status")),
            created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
            opens_at: chrono::DateTime::parse_from_rfc3339(row.get("opens_at"))
                .unwrap()
                .into(),
            closes_at: chrono::DateTime::parse_from_rfc3339(row.get("closes_at"))
                .unwrap()
                .into(),
            starting_balance: row.get("starting_balance"),
            invite_code: row.get("invite_code"),
            created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                .unwrap()
                .into(),
        })
    }

    async fn get_market_by_invite_code(&self, code: &str) -> DbResult<Market> {
        let row = sqlx::query("SELECT * FROM markets WHERE invite_code = ?")
            .bind(code)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| DbError::NotFound("Market not found".to_string()))?;

        Ok(Market {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            name: row.get("name"),
            status: deserialize_market_status(row.get("status")),
            created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
            opens_at: chrono::DateTime::parse_from_rfc3339(row.get("opens_at"))
                .unwrap()
                .into(),
            closes_at: chrono::DateTime::parse_from_rfc3339(row.get("closes_at"))
                .unwrap()
                .into(),
            starting_balance: row.get("starting_balance"),
            invite_code: row.get("invite_code"),
            created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                .unwrap()
                .into(),
        })
    }

    async fn update_market_status(&self, id: Uuid, status: MarketStatus) -> DbResult<()> {
        sqlx::query("UPDATE markets SET status = ? WHERE id = ?")
            .bind(serialize_market_status(status))
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn create_user(&self, user: User) -> DbResult<User> {
        sqlx::query(
            r#"
            INSERT INTO users (id, market_id, device_id, display_name, avatar, balance, is_admin, joined_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id.to_string())
        .bind(user.market_id.to_string())
        .bind(&user.device_id)
        .bind(&user.display_name)
        .bind(&user.avatar)
        .bind(user.balance)
        .bind(user.is_admin as i64)
        .bind(user.joined_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(user)
    }

    async fn get_user(&self, id: Uuid) -> DbResult<User> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| DbError::NotFound("User not found".to_string()))?;

        Ok(User {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
            device_id: row.get("device_id"),
            display_name: row.get("display_name"),
            avatar: row.get("avatar"),
            balance: row.get("balance"),
            is_admin: row.get::<i64, _>("is_admin") != 0,
            joined_at: chrono::DateTime::parse_from_rfc3339(row.get("joined_at"))
                .unwrap()
                .into(),
        })
    }

    async fn get_user_by_device_id(&self, market_id: Uuid, device_id: &str) -> DbResult<User> {
        let row = sqlx::query("SELECT * FROM users WHERE market_id = ? AND device_id = ?")
            .bind(market_id.to_string())
            .bind(device_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| DbError::NotFound("User not found".to_string()))?;

        Ok(User {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
            device_id: row.get("device_id"),
            display_name: row.get("display_name"),
            avatar: row.get("avatar"),
            balance: row.get("balance"),
            is_admin: row.get::<i64, _>("is_admin") != 0,
            joined_at: chrono::DateTime::parse_from_rfc3339(row.get("joined_at"))
                .unwrap()
                .into(),
        })
    }

    async fn get_users_in_market(&self, market_id: Uuid) -> DbResult<Vec<User>> {
        let rows = sqlx::query("SELECT * FROM users WHERE market_id = ?")
            .bind(market_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| User {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
                device_id: row.get("device_id"),
                display_name: row.get("display_name"),
                avatar: row.get("avatar"),
                balance: row.get("balance"),
                is_admin: row.get::<i64, _>("is_admin") != 0,
                joined_at: chrono::DateTime::parse_from_rfc3339(row.get("joined_at"))
                    .unwrap()
                    .into(),
            })
            .collect())
    }

    async fn update_user_balance(&self, user_id: Uuid, new_balance: i64) -> DbResult<()> {
        sqlx::query("UPDATE users SET balance = ? WHERE id = ?")
            .bind(new_balance)
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_markets_by_device_id(&self, device_id: &str) -> DbResult<Vec<(Market, User)>> {
        let rows = sqlx::query(
            r#"
            SELECT
                m.id as market_id, m.name, m.status, m.created_by, m.opens_at, m.closes_at,
                m.starting_balance, m.invite_code, m.created_at,
                u.id as user_id, u.market_id as u_market_id, u.device_id, u.display_name,
                u.avatar, u.balance, u.is_admin, u.joined_at
            FROM users u
            JOIN markets m ON u.market_id = m.id
            WHERE u.device_id = ?
            ORDER BY u.joined_at DESC
            LIMIT 10
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| {
                let market = Market {
                    id: Uuid::parse_str(row.get("market_id")).unwrap(),
                    name: row.get("name"),
                    status: deserialize_market_status(row.get("status")),
                    created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
                    opens_at: chrono::DateTime::parse_from_rfc3339(row.get("opens_at"))
                        .unwrap()
                        .into(),
                    closes_at: chrono::DateTime::parse_from_rfc3339(row.get("closes_at"))
                        .unwrap()
                        .into(),
                    starting_balance: row.get("starting_balance"),
                    invite_code: row.get("invite_code"),
                    created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                        .unwrap()
                        .into(),
                };
                let user = User {
                    id: Uuid::parse_str(row.get("user_id")).unwrap(),
                    market_id: Uuid::parse_str(row.get("u_market_id")).unwrap(),
                    device_id: row.get("device_id"),
                    display_name: row.get("display_name"),
                    avatar: row.get("avatar"),
                    balance: row.get("balance"),
                    is_admin: row.get::<i64, _>("is_admin") != 0,
                    joined_at: chrono::DateTime::parse_from_rfc3339(row.get("joined_at"))
                        .unwrap()
                        .into(),
                };
                (market, user)
            })
            .collect())
    }

    async fn create_bet(&self, bet: Bet) -> DbResult<Bet> {
        sqlx::query(
            r#"
            INSERT INTO bets (id, market_id, subject_user_id, created_by, description, initial_odds, status, yes_pool, no_pool, created_at, resolved_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(bet.id.to_string())
        .bind(bet.market_id.to_string())
        .bind(bet.subject_user_id.to_string())
        .bind(bet.created_by.to_string())
        .bind(&bet.description)
        .bind(&bet.initial_odds)
        .bind(serialize_bet_status(bet.status))
        .bind(bet.yes_pool)
        .bind(bet.no_pool)
        .bind(bet.created_at.to_rfc3339())
        .bind(bet.resolved_at.map(|d| d.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(bet)
    }

    async fn get_bet(&self, id: Uuid) -> DbResult<Bet> {
        let row = sqlx::query("SELECT * FROM bets WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|_| DbError::NotFound("Bet not found".to_string()))?;

        Ok(Bet {
            id: Uuid::parse_str(row.get("id")).unwrap(),
            market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
            subject_user_id: Uuid::parse_str(row.get("subject_user_id")).unwrap(),
            created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
            description: row.get("description"),
            initial_odds: row.get("initial_odds"),
            status: deserialize_bet_status(row.get("status")),
            yes_pool: row.get("yes_pool"),
            no_pool: row.get("no_pool"),
            created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                .unwrap()
                .into(),
            resolved_at: row
                .get::<Option<String>, _>("resolved_at")
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
        })
    }

    async fn get_bets_in_market(&self, market_id: Uuid) -> DbResult<Vec<Bet>> {
        let rows = sqlx::query("SELECT * FROM bets WHERE market_id = ?")
            .bind(market_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| Bet {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
                subject_user_id: Uuid::parse_str(row.get("subject_user_id")).unwrap(),
                created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
                description: row.get("description"),
                initial_odds: row.get("initial_odds"),
                status: deserialize_bet_status(row.get("status")),
                yes_pool: row.get("yes_pool"),
                no_pool: row.get("no_pool"),
                created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                    .unwrap()
                    .into(),
                resolved_at: row
                    .get::<Option<String>, _>("resolved_at")
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
            })
            .collect())
    }

    async fn get_bets_for_user(
        &self,
        market_id: Uuid,
        viewing_user_id: Uuid,
    ) -> DbResult<Vec<BetView>> {
        let bets = self.get_bets_in_market(market_id).await?;
        Ok(bets
            .iter()
            .map(|bet| bet.to_view(viewing_user_id))
            .collect())
    }

    async fn get_pending_bets(&self, market_id: Uuid) -> DbResult<Vec<Bet>> {
        let rows = sqlx::query("SELECT * FROM bets WHERE market_id = ? AND status = ?")
            .bind(market_id.to_string())
            .bind(serialize_bet_status(BetStatus::Pending))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| Bet {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
                subject_user_id: Uuid::parse_str(row.get("subject_user_id")).unwrap(),
                created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
                description: row.get("description"),
                initial_odds: row.get("initial_odds"),
                status: deserialize_bet_status(row.get("status")),
                yes_pool: row.get("yes_pool"),
                no_pool: row.get("no_pool"),
                created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                    .unwrap()
                    .into(),
                resolved_at: row
                    .get::<Option<String>, _>("resolved_at")
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
            })
            .collect())
    }

    async fn update_bet_status(&self, bet_id: Uuid, status: BetStatus) -> DbResult<()> {
        let resolved_at = match status {
            BetStatus::ResolvedYes | BetStatus::ResolvedNo => Some(chrono::Utc::now().to_rfc3339()),
            _ => None,
        };

        sqlx::query("UPDATE bets SET status = ?, resolved_at = ? WHERE id = ?")
            .bind(serialize_bet_status(status))
            .bind(resolved_at)
            .bind(bet_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn update_bet_pools(&self, bet_id: Uuid, yes_pool: i64, no_pool: i64) -> DbResult<()> {
        sqlx::query("UPDATE bets SET yes_pool = ?, no_pool = ? WHERE id = ?")
            .bind(yes_pool)
            .bind(no_pool)
            .bind(bet_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn create_wager(&self, wager: Wager) -> DbResult<Wager> {
        sqlx::query(
            r#"
            INSERT INTO wagers (id, bet_id, user_id, side, amount, placed_at, yes_pool_after, no_pool_after, probability_after)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(wager.id.to_string())
        .bind(wager.bet_id.to_string())
        .bind(wager.user_id.to_string())
        .bind(serialize_side(wager.side))
        .bind(wager.amount)
        .bind(wager.placed_at.to_rfc3339())
        .bind(wager.yes_pool_after)
        .bind(wager.no_pool_after)
        .bind(wager.probability_after)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(wager)
    }

    async fn get_wagers_for_bet(&self, bet_id: Uuid) -> DbResult<Vec<Wager>> {
        let rows = sqlx::query("SELECT * FROM wagers WHERE bet_id = ? ORDER BY placed_at")
            .bind(bet_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| Wager {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                bet_id: Uuid::parse_str(row.get("bet_id")).unwrap(),
                user_id: Uuid::parse_str(row.get("user_id")).unwrap(),
                side: deserialize_side(row.get("side")),
                amount: row.get("amount"),
                placed_at: chrono::DateTime::parse_from_rfc3339(row.get("placed_at"))
                    .unwrap()
                    .into(),
                yes_pool_after: row.get("yes_pool_after"),
                no_pool_after: row.get("no_pool_after"),
                probability_after: row.get("probability_after"),
            })
            .collect())
    }

    async fn get_wagers_for_user(&self, user_id: Uuid) -> DbResult<Vec<Wager>> {
        let rows = sqlx::query("SELECT * FROM wagers WHERE user_id = ? ORDER BY placed_at")
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| Wager {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                bet_id: Uuid::parse_str(row.get("bet_id")).unwrap(),
                user_id: Uuid::parse_str(row.get("user_id")).unwrap(),
                side: deserialize_side(row.get("side")),
                amount: row.get("amount"),
                placed_at: chrono::DateTime::parse_from_rfc3339(row.get("placed_at"))
                    .unwrap()
                    .into(),
                yes_pool_after: row.get("yes_pool_after"),
                no_pool_after: row.get("no_pool_after"),
                probability_after: row.get("probability_after"),
            })
            .collect())
    }

    async fn get_bets_about_user(&self, user_id: Uuid) -> DbResult<Vec<Bet>> {
        let rows = sqlx::query("SELECT * FROM bets WHERE subject_user_id = ?")
            .bind(user_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| Bet {
                id: Uuid::parse_str(row.get("id")).unwrap(),
                market_id: Uuid::parse_str(row.get("market_id")).unwrap(),
                subject_user_id: Uuid::parse_str(row.get("subject_user_id")).unwrap(),
                created_by: Uuid::parse_str(row.get("created_by")).unwrap(),
                description: row.get("description"),
                initial_odds: row.get("initial_odds"),
                status: deserialize_bet_status(row.get("status")),
                yes_pool: row.get("yes_pool"),
                no_pool: row.get("no_pool"),
                created_at: chrono::DateTime::parse_from_rfc3339(row.get("created_at"))
                    .unwrap()
                    .into(),
                resolved_at: row
                    .get::<Option<String>, _>("resolved_at")
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
            })
            .collect())
    }
}
