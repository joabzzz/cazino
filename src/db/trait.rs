/// Database abstraction trait
///
/// This trait defines all database operations needed by the application.
/// We can swap implementations (SQLite, Supabase, etc.) without changing business logic.
use crate::domain::models::{Bet, BetStatus, BetView, Market, MarketStatus, User, Wager};
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Internal(String),

    #[error("Constraint violation: {0}")]
    Constraint(String),
}

pub type DbResult<T> = Result<T, DbError>;

#[cfg(feature = "wasm")]
pub trait DatabaseMarker {}
#[cfg(feature = "wasm")]
impl<T> DatabaseMarker for T {}

#[cfg(not(feature = "wasm"))]
pub trait DatabaseMarker: Send + Sync {}
#[cfg(not(feature = "wasm"))]
impl<T: Send + Sync> DatabaseMarker for T {}

#[cfg_attr(feature = "wasm", async_trait(?Send))]
#[cfg_attr(not(feature = "wasm"), async_trait)]
pub trait Database: DatabaseMarker {
    // ===== Market Operations =====

    async fn create_market(&self, market: Market) -> DbResult<Market>;

    async fn get_market(&self, id: Uuid) -> DbResult<Market>;

    async fn get_market_by_invite_code(&self, code: &str) -> DbResult<Market>;

    async fn update_market_status(&self, id: Uuid, status: MarketStatus) -> DbResult<()>;

    // ===== User Operations =====

    async fn create_user(&self, user: User) -> DbResult<User>;

    async fn get_user(&self, id: Uuid) -> DbResult<User>;

    async fn get_user_by_device_id(&self, market_id: Uuid, device_id: &str) -> DbResult<User>;

    async fn get_users_in_market(&self, market_id: Uuid) -> DbResult<Vec<User>>;

    async fn update_user_balance(&self, user_id: Uuid, new_balance: i64) -> DbResult<()>;

    // ===== Bet Operations =====

    async fn create_bet(&self, bet: Bet) -> DbResult<Bet>;

    async fn get_bet(&self, id: Uuid) -> DbResult<Bet>;

    async fn get_bets_in_market(&self, market_id: Uuid) -> DbResult<Vec<Bet>>;

    /// Get bets with visibility filtering for a specific user
    /// This applies the "hidden bet" rule: users can't see bets about themselves
    async fn get_bets_for_user(
        &self,
        market_id: Uuid,
        viewing_user_id: Uuid,
    ) -> DbResult<Vec<BetView>>;

    async fn get_pending_bets(&self, market_id: Uuid) -> DbResult<Vec<Bet>>;

    async fn update_bet_status(&self, bet_id: Uuid, status: BetStatus) -> DbResult<()>;

    async fn update_bet_pools(&self, bet_id: Uuid, yes_pool: i64, no_pool: i64) -> DbResult<()>;

    // ===== Wager Operations =====

    async fn create_wager(&self, wager: Wager) -> DbResult<Wager>;

    async fn get_wagers_for_bet(&self, bet_id: Uuid) -> DbResult<Vec<Wager>>;

    #[allow(dead_code)]
    async fn get_wagers_for_user(&self, user_id: Uuid) -> DbResult<Vec<Wager>>;

    // ===== Reveal Operations (end of market) =====

    /// Get all bets about a specific user (for reveal screen)
    async fn get_bets_about_user(&self, user_id: Uuid) -> DbResult<Vec<Bet>>;
}
