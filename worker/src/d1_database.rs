/// D1 implementation of the Database trait for Cloudflare Workers
use async_trait::async_trait;
use cazino::db::{Database, DbError, DbResult};
use cazino::domain::models::{Bet, BetStatus, BetView, Market, MarketStatus, Side, User, Wager};
use serde::Deserialize;
use uuid::Uuid;
use wasm_bindgen::JsValue;
use worker::*;

pub struct D1Database {
    db: worker::D1Database,
}

impl D1Database {
    pub fn new(db: worker::D1Database) -> Self {
        Self { db }
    }
}

// D1Database doesn't implement Clone, so we can't derive Clone
// Instead, we'll pass Arc<D1Database> where needed

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

// D1 row deserializers
#[derive(Debug, Deserialize)]
struct MarketRow {
    id: String,
    name: String,
    status: String,
    created_by: String,
    opens_at: String,
    closes_at: String,
    starting_balance: i64,
    invite_code: String,
    created_at: String,
}

impl MarketRow {
    fn into_market(self) -> Market {
        Market {
            id: Uuid::parse_str(&self.id).unwrap(),
            name: self.name,
            status: deserialize_market_status(&self.status),
            created_by: Uuid::parse_str(&self.created_by).unwrap(),
            opens_at: chrono::DateTime::parse_from_rfc3339(&self.opens_at)
                .unwrap()
                .into(),
            closes_at: chrono::DateTime::parse_from_rfc3339(&self.closes_at)
                .unwrap()
                .into(),
            starting_balance: self.starting_balance,
            invite_code: self.invite_code,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .unwrap()
                .into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UserRow {
    id: String,
    market_id: String,
    device_id: String,
    display_name: String,
    avatar: String,
    balance: i64,
    is_admin: i64,
    joined_at: String,
}

impl UserRow {
    fn into_user(self) -> User {
        User {
            id: Uuid::parse_str(&self.id).unwrap(),
            market_id: Uuid::parse_str(&self.market_id).unwrap(),
            device_id: self.device_id,
            display_name: self.display_name,
            avatar: self.avatar,
            balance: self.balance,
            is_admin: self.is_admin != 0,
            joined_at: chrono::DateTime::parse_from_rfc3339(&self.joined_at)
                .unwrap()
                .into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BetRow {
    id: String,
    market_id: String,
    subject_user_id: String,
    created_by: String,
    description: String,
    initial_odds: String,
    status: String,
    yes_pool: i64,
    no_pool: i64,
    created_at: String,
    resolved_at: Option<String>,
}

impl BetRow {
    fn into_bet(self) -> Bet {
        Bet {
            id: Uuid::parse_str(&self.id).unwrap(),
            market_id: Uuid::parse_str(&self.market_id).unwrap(),
            subject_user_id: Uuid::parse_str(&self.subject_user_id).unwrap(),
            created_by: Uuid::parse_str(&self.created_by).unwrap(),
            description: self.description,
            initial_odds: self.initial_odds,
            status: deserialize_bet_status(&self.status),
            yes_pool: self.yes_pool,
            no_pool: self.no_pool,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .unwrap()
                .into(),
            resolved_at: self
                .resolved_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().into()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WagerRow {
    id: String,
    bet_id: String,
    user_id: String,
    side: String,
    amount: i64,
    placed_at: String,
    yes_pool_after: i64,
    no_pool_after: i64,
    probability_after: f64,
}

impl WagerRow {
    fn into_wager(self) -> Wager {
        Wager {
            id: Uuid::parse_str(&self.id).unwrap(),
            bet_id: Uuid::parse_str(&self.bet_id).unwrap(),
            user_id: Uuid::parse_str(&self.user_id).unwrap(),
            side: deserialize_side(&self.side),
            amount: self.amount,
            placed_at: chrono::DateTime::parse_from_rfc3339(&self.placed_at)
                .unwrap()
                .into(),
            yes_pool_after: self.yes_pool_after,
            no_pool_after: self.no_pool_after,
            probability_after: self.probability_after,
        }
    }
}

#[async_trait(?Send)]
impl Database for D1Database {
    async fn create_market(&self, market: Market) -> DbResult<Market> {
        self.db
            .prepare(
                r#"
                INSERT INTO markets (id, name, status, created_by, opens_at, closes_at, starting_balance, invite_code, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
            )
            .bind(&[
                JsValue::from_str(&market.id.to_string()),
                JsValue::from_str(&market.name),
                JsValue::from_str(&serialize_market_status(market.status)),
                JsValue::from_str(&market.created_by.to_string()),
                JsValue::from_str(&market.opens_at.to_rfc3339()),
                JsValue::from_str(&market.closes_at.to_rfc3339()),
                JsValue::from_f64(market.starting_balance as f64),
                JsValue::from_str(&market.invite_code),
                JsValue::from_str(&market.created_at.to_rfc3339()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to insert market: {}", e)))?;

        Ok(market)
    }

    async fn get_market(&self, id: Uuid) -> DbResult<Market> {
        let result = self
            .db
            .prepare("SELECT * FROM markets WHERE id = ?1")
            .bind(&[JsValue::from_str(&id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .first::<MarketRow>(None)
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| DbError::NotFound("Market not found".to_string()))?;

        Ok(result.into_market())
    }

    async fn get_market_by_invite_code(&self, code: &str) -> DbResult<Market> {
        let result = self
            .db
            .prepare("SELECT * FROM markets WHERE invite_code = ?1")
            .bind(&[JsValue::from_str(code)])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .first::<MarketRow>(None)
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| DbError::NotFound("Market not found".to_string()))?;

        Ok(result.into_market())
    }

    async fn update_market_status(&self, id: Uuid, status: MarketStatus) -> DbResult<()> {
        self.db
            .prepare("UPDATE markets SET status = ?1 WHERE id = ?2")
            .bind(&[
                JsValue::from_str(&serialize_market_status(status)),
                JsValue::from_str(&id.to_string()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to update market status: {}", e)))?;

        Ok(())
    }

    async fn create_user(&self, user: User) -> DbResult<User> {
        self.db
            .prepare(
                r#"
                INSERT INTO users (id, market_id, device_id, display_name, avatar, balance, is_admin, joined_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )
            .bind(&[
                JsValue::from_str(&user.id.to_string()),
                JsValue::from_str(&user.market_id.to_string()),
                JsValue::from_str(&user.device_id),
                JsValue::from_str(&user.display_name),
                JsValue::from_str(&user.avatar),
                JsValue::from_f64(user.balance as f64),
                JsValue::from_f64(if user.is_admin { 1.0 } else { 0.0 }),
                JsValue::from_str(&user.joined_at.to_rfc3339()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to insert user: {}", e)))?;

        Ok(user)
    }

    async fn get_user(&self, id: Uuid) -> DbResult<User> {
        let result = self
            .db
            .prepare("SELECT * FROM users WHERE id = ?1")
            .bind(&[JsValue::from_str(&id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .first::<UserRow>(None)
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| DbError::NotFound("User not found".to_string()))?;

        Ok(result.into_user())
    }

    async fn get_user_by_device_id(&self, market_id: Uuid, device_id: &str) -> DbResult<User> {
        let result = self
            .db
            .prepare("SELECT * FROM users WHERE market_id = ?1 AND device_id = ?2")
            .bind(&[
                JsValue::from_str(&market_id.to_string()),
                JsValue::from_str(device_id),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .first::<UserRow>(None)
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| DbError::NotFound("User not found".to_string()))?;

        Ok(result.into_user())
    }

    async fn get_users_in_market(&self, market_id: Uuid) -> DbResult<Vec<User>> {
        let results = self
            .db
            .prepare("SELECT * FROM users WHERE market_id = ?1")
            .bind(&[JsValue::from_str(&market_id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let users: Vec<User> = results
            .results::<UserRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize users: {}", e)))?
            .into_iter()
            .map(|row| row.into_user())
            .collect();

        Ok(users)
    }

    async fn update_user_balance(&self, user_id: Uuid, new_balance: i64) -> DbResult<()> {
        self.db
            .prepare("UPDATE users SET balance = ?1 WHERE id = ?2")
            .bind(&[
                JsValue::from_f64(new_balance as f64),
                JsValue::from_str(&user_id.to_string()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to update user balance: {}", e)))?;

        Ok(())
    }

    async fn create_bet(&self, bet: Bet) -> DbResult<Bet> {
        self.db
            .prepare(
                r#"
                INSERT INTO bets (id, market_id, subject_user_id, created_by, description, initial_odds, status, yes_pool, no_pool, created_at, resolved_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
            )
            .bind(&[
                JsValue::from_str(&bet.id.to_string()),
                JsValue::from_str(&bet.market_id.to_string()),
                JsValue::from_str(&bet.subject_user_id.to_string()),
                JsValue::from_str(&bet.created_by.to_string()),
                JsValue::from_str(&bet.description),
                JsValue::from_str(&bet.initial_odds),
                JsValue::from_str(&serialize_bet_status(bet.status)),
                JsValue::from_f64(bet.yes_pool as f64),
                JsValue::from_f64(bet.no_pool as f64),
                JsValue::from_str(&bet.created_at.to_rfc3339()),
                bet.resolved_at.map(|d| JsValue::from_str(&d.to_rfc3339())).unwrap_or(JsValue::null()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to insert bet: {}", e)))?;

        Ok(bet)
    }

    async fn get_bet(&self, id: Uuid) -> DbResult<Bet> {
        let result = self
            .db
            .prepare("SELECT * FROM bets WHERE id = ?1")
            .bind(&[JsValue::from_str(&id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .first::<BetRow>(None)
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| DbError::NotFound("Bet not found".to_string()))?;

        Ok(result.into_bet())
    }

    async fn get_bets_in_market(&self, market_id: Uuid) -> DbResult<Vec<Bet>> {
        let results = self
            .db
            .prepare("SELECT * FROM bets WHERE market_id = ?1")
            .bind(&[JsValue::from_str(&market_id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let bets: Vec<Bet> = results
            .results::<BetRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize bets: {}", e)))?
            .into_iter()
            .map(|row| row.into_bet())
            .collect();

        Ok(bets)
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
        let results = self
            .db
            .prepare("SELECT * FROM bets WHERE market_id = ?1 AND status = ?2")
            .bind(&[
                JsValue::from_str(&market_id.to_string()),
                JsValue::from_str(&serialize_bet_status(BetStatus::Pending)),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let bets: Vec<Bet> = results
            .results::<BetRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize bets: {}", e)))?
            .into_iter()
            .map(|row| row.into_bet())
            .collect();

        Ok(bets)
    }

    async fn update_bet_status(&self, bet_id: Uuid, status: BetStatus) -> DbResult<()> {
        let resolved_at = match status {
            BetStatus::ResolvedYes | BetStatus::ResolvedNo => {
                JsValue::from_str(&chrono::Utc::now().to_rfc3339())
            }
            _ => JsValue::null(),
        };

        self.db
            .prepare("UPDATE bets SET status = ?1, resolved_at = ?2 WHERE id = ?3")
            .bind(&[
                JsValue::from_str(&serialize_bet_status(status)),
                resolved_at,
                JsValue::from_str(&bet_id.to_string()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to update bet status: {}", e)))?;

        Ok(())
    }

    async fn update_bet_pools(&self, bet_id: Uuid, yes_pool: i64, no_pool: i64) -> DbResult<()> {
        self.db
            .prepare("UPDATE bets SET yes_pool = ?1, no_pool = ?2 WHERE id = ?3")
            .bind(&[
                JsValue::from_f64(yes_pool as f64),
                JsValue::from_f64(no_pool as f64),
                JsValue::from_str(&bet_id.to_string()),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to update bet pools: {}", e)))?;

        Ok(())
    }

    async fn create_wager(&self, wager: Wager) -> DbResult<Wager> {
        self.db
            .prepare(
                r#"
                INSERT INTO wagers (id, bet_id, user_id, side, amount, placed_at, yes_pool_after, no_pool_after, probability_after)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
            )
            .bind(&[
                JsValue::from_str(&wager.id.to_string()),
                JsValue::from_str(&wager.bet_id.to_string()),
                JsValue::from_str(&wager.user_id.to_string()),
                JsValue::from_str(&serialize_side(wager.side)),
                JsValue::from_f64(wager.amount as f64),
                JsValue::from_str(&wager.placed_at.to_rfc3339()),
                JsValue::from_f64(wager.yes_pool_after as f64),
                JsValue::from_f64(wager.no_pool_after as f64),
                JsValue::from_f64(wager.probability_after),
            ])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .run()
            .await
            .map_err(|e| DbError::Internal(format!("Failed to insert wager: {}", e)))?;

        Ok(wager)
    }

    async fn get_wagers_for_bet(&self, bet_id: Uuid) -> DbResult<Vec<Wager>> {
        let results = self
            .db
            .prepare("SELECT * FROM wagers WHERE bet_id = ?1 ORDER BY placed_at")
            .bind(&[JsValue::from_str(&bet_id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let wagers: Vec<Wager> = results
            .results::<WagerRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize wagers: {}", e)))?
            .into_iter()
            .map(|row| row.into_wager())
            .collect();

        Ok(wagers)
    }

    async fn get_wagers_for_user(&self, user_id: Uuid) -> DbResult<Vec<Wager>> {
        let results = self
            .db
            .prepare("SELECT * FROM wagers WHERE user_id = ?1 ORDER BY placed_at")
            .bind(&[JsValue::from_str(&user_id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let wagers: Vec<Wager> = results
            .results::<WagerRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize wagers: {}", e)))?
            .into_iter()
            .map(|row| row.into_wager())
            .collect();

        Ok(wagers)
    }

    async fn get_bets_about_user(&self, user_id: Uuid) -> DbResult<Vec<Bet>> {
        let results = self
            .db
            .prepare("SELECT * FROM bets WHERE subject_user_id = ?1")
            .bind(&[JsValue::from_str(&user_id.to_string())])
            .map_err(|e| DbError::Internal(format!("Failed to bind: {}", e)))?
            .all()
            .await
            .map_err(|e| DbError::Internal(format!("Query failed: {}", e)))?;

        let bets: Vec<Bet> = results
            .results::<BetRow>()
            .map_err(|e| DbError::Internal(format!("Failed to deserialize bets: {}", e)))?
            .into_iter()
            .map(|row| row.into_bet())
            .collect();

        Ok(bets)
    }
}
