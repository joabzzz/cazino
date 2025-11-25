/// Service layer - orchestrates domain logic and database operations
/// This is where transactions and complex business flows live
use crate::db::{Database, DbResult};
use crate::domain::models::{Bet, BetStatus, BetView, Market, MarketStatus, Side, User, Wager};
use crate::domain::{parimutuel, rules};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Parameters for creating a new market
pub struct CreateMarketParams {
    pub name: String,
    pub admin_device_id: String,
    pub admin_name: String,
    pub admin_avatar: String,
    pub starting_balance: i64,
    pub duration_hours: i64,
    pub custom_invite_code: Option<String>,
}

pub struct CazinoService<D: Database> {
    db: Arc<D>,
}

impl<D: Database> CazinoService<D> {
    pub fn new(db: Arc<D>) -> Self {
        Self { db }
    }

    /// Create a new market
    pub async fn create_market(&self, params: CreateMarketParams) -> DbResult<(Market, User)> {
        let now = Utc::now();
        let market_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        // Use custom invite code if provided, otherwise generate one
        // Note: Custom codes must be unique and 6 characters uppercase alphanumeric
        let invite_code = params
            .custom_invite_code
            .filter(|code| {
                code.len() == 6
                    && code.chars().all(|c| {
                        c.is_ascii_alphanumeric() && (c.is_ascii_uppercase() || c.is_ascii_digit())
                    })
            })
            .unwrap_or_else(generate_invite_code);

        let market = Market {
            id: market_id,
            name: params.name,
            status: MarketStatus::Draft,
            created_by: admin_id,
            opens_at: now,
            closes_at: now + chrono::Duration::hours(params.duration_hours),
            starting_balance: params.starting_balance,
            invite_code,
            created_at: now,
        };

        let admin = User {
            id: admin_id,
            market_id,
            device_id: params.admin_device_id,
            display_name: params.admin_name,
            avatar: params.admin_avatar,
            balance: params.starting_balance,
            is_admin: true,
            joined_at: now,
        };

        let market = self.db.create_market(market).await?;
        let admin = self.db.create_user(admin).await?;

        Ok((market, admin))
    }

    /// Join an existing market
    pub async fn join_market(
        &self,
        invite_code: String,
        device_id: String,
        display_name: String,
        avatar: String,
    ) -> DbResult<(Market, User)> {
        let market = self.db.get_market_by_invite_code(&invite_code).await?;

        // Check if user already exists (returning user)
        if let Ok(existing_user) = self.db.get_user_by_device_id(market.id, &device_id).await {
            return Ok((market, existing_user));
        }

        // Create new user
        let user = User {
            id: Uuid::new_v4(),
            market_id: market.id,
            device_id,
            display_name,
            avatar,
            balance: market.starting_balance,
            is_admin: false,
            joined_at: Utc::now(),
        };

        let user = self.db.create_user(user).await?;

        Ok((market, user))
    }

    /// Create a new bet (goes to pending approval)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_bet(
        &self,
        market_id: Uuid,
        creator_id: Uuid,
        subject_user_id: Uuid,
        description: String,
        initial_odds: String,
        opening_wager: i64,
        hide_from_subject: bool,
    ) -> DbResult<Bet> {
        let market = self.db.get_market(market_id).await?;
        let creator = self.db.get_user(creator_id).await?;

        // Validate
        rules::validate_bet_creation(&market, &creator, subject_user_id, opening_wager)
            .map_err(|e| crate::db::DbError::Constraint(e.to_string()))?;

        // Parse initial odds to determine starting pools
        let (yes_pool, no_pool) = parimutuel::parse_initial_odds(&initial_odds, opening_wager)
            .ok_or_else(|| crate::db::DbError::Constraint("Invalid odds format".to_string()))?;

        let bet = Bet {
            id: Uuid::new_v4(),
            market_id,
            subject_user_id,
            created_by: creator_id,
            description,
            initial_odds,
            status: BetStatus::Active,
            yes_pool,
            no_pool,
            hide_from_subject,
            created_at: Utc::now(),
            resolved_at: None,
        };

        let bet = self.db.create_bet(bet).await?;

        // Create a wager record for the creator's opening bet (always YES)
        let opening_wager_record = Wager {
            id: Uuid::new_v4(),
            bet_id: bet.id,
            user_id: creator_id,
            side: Side::Yes,
            amount: opening_wager,
            placed_at: Utc::now(),
            yes_pool_after: yes_pool,
            no_pool_after: no_pool,
            probability_after: parimutuel::calculate_probability(yes_pool, no_pool),
        };

        self.db.create_wager(opening_wager_record).await?;

        // Deduct opening wager from creator's balance
        self.db
            .update_user_balance(creator_id, creator.balance - opening_wager)
            .await?;

        Ok(bet)
    }

    /// Approve a bet (admin only) - moves from Pending to Active
    pub async fn approve_bet(&self, bet_id: Uuid, admin_id: Uuid) -> DbResult<Bet> {
        let admin = self.db.get_user(admin_id).await?;

        // Validate admin
        rules::validate_bet_approval(&admin)
            .map_err(|e| crate::db::DbError::Constraint(e.to_string()))?;

        self.db.update_bet_status(bet_id, BetStatus::Active).await?;

        self.db.get_bet(bet_id).await
    }

    /// Place a wager on a bet
    pub async fn place_wager(
        &self,
        bet_id: Uuid,
        user_id: Uuid,
        side: Side,
        amount: i64,
    ) -> DbResult<Wager> {
        let bet = self.db.get_bet(bet_id).await?;
        let market = self.db.get_market(bet.market_id).await?;
        let user = self.db.get_user(user_id).await?;

        // Validate wager
        rules::validate_wager(&market, &bet, &user, amount)
            .map_err(|e| crate::db::DbError::Constraint(e.to_string()))?;

        // Calculate new pools and probability
        let (yes_pool_after, no_pool_after, _) =
            parimutuel::calculate_potential_payout(bet.yes_pool, bet.no_pool, side, amount);

        let probability_after = parimutuel::calculate_probability(yes_pool_after, no_pool_after);

        // Create wager
        let wager = Wager {
            id: Uuid::new_v4(),
            bet_id,
            user_id,
            side,
            amount,
            placed_at: Utc::now(),
            yes_pool_after,
            no_pool_after,
            probability_after,
        };

        let wager = self.db.create_wager(wager).await?;

        // Update bet pools
        self.db
            .update_bet_pools(bet_id, yes_pool_after, no_pool_after)
            .await?;

        // Deduct from user balance
        self.db
            .update_user_balance(user_id, user.balance - amount)
            .await?;

        Ok(wager)
    }

    /// Resolve a bet (admin only)
    pub async fn resolve_bet(
        &self,
        bet_id: Uuid,
        admin_id: Uuid,
        outcome: Side,
    ) -> DbResult<Vec<(Uuid, i64)>> {
        let bet = self.db.get_bet(bet_id).await?;
        let market = self.db.get_market(bet.market_id).await?;
        let admin = self.db.get_user(admin_id).await?;

        // Validate resolution
        rules::validate_bet_resolution(&market, &bet, &admin)
            .map_err(|e| crate::db::DbError::Constraint(e.to_string()))?;

        // Update bet status
        let new_status = match outcome {
            Side::Yes => BetStatus::ResolvedYes,
            Side::No => BetStatus::ResolvedNo,
        };

        self.db.update_bet_status(bet_id, new_status).await?;

        // Get updated bet
        let bet = self.db.get_bet(bet_id).await?;

        // Calculate payouts
        let wagers = self.db.get_wagers_for_bet(bet_id).await?;
        let payouts = parimutuel::calculate_payouts(&bet, &wagers);

        // Update user balances
        for (user_id, payout) in &payouts {
            let user = self.db.get_user(*user_id).await?;
            self.db
                .update_user_balance(*user_id, user.balance + payout)
                .await?;
        }

        Ok(payouts)
    }

    /// Get all bets in a market (with visibility filtering)
    pub async fn get_bets(&self, market_id: Uuid, viewing_user_id: Uuid) -> DbResult<Vec<BetView>> {
        self.db.get_bets_for_user(market_id, viewing_user_id).await
    }

    /// Get pending bets for admin approval
    pub async fn get_pending_bets(&self, market_id: Uuid) -> DbResult<Vec<Bet>> {
        self.db.get_pending_bets(market_id).await
    }

    /// Get probability chart data for a bet
    pub async fn get_probability_chart(
        &self,
        bet_id: Uuid,
    ) -> DbResult<Vec<crate::domain::models::ProbabilityPoint>> {
        let wagers = self.db.get_wagers_for_bet(bet_id).await?;

        Ok(wagers
            .iter()
            .map(|w| crate::domain::models::ProbabilityPoint {
                timestamp: w.placed_at,
                yes_probability: w.probability_after,
            })
            .collect())
    }

    /// Get all users in a market (for leaderboard)
    pub async fn get_users(&self, market_id: Uuid) -> DbResult<Vec<User>> {
        self.db.get_users_in_market(market_id).await
    }

    /// Get bets about a specific user (for reveal screen)
    pub async fn get_bets_about_user(&self, user_id: Uuid) -> DbResult<Vec<Bet>> {
        self.db.get_bets_about_user(user_id).await
    }

    /// Open a market for betting
    pub async fn open_market(&self, market_id: Uuid, admin_id: Uuid) -> DbResult<()> {
        let admin = self.db.get_user(admin_id).await?;

        if !admin.is_admin {
            return Err(crate::db::DbError::Constraint("Admin only".to_string()));
        }

        self.db
            .update_market_status(market_id, MarketStatus::Open)
            .await
    }

    /// Close a market (end betting period)
    pub async fn close_market(&self, market_id: Uuid, admin_id: Uuid) -> DbResult<()> {
        let admin = self.db.get_user(admin_id).await?;

        if !admin.is_admin {
            return Err(crate::db::DbError::Constraint("Admin only".to_string()));
        }

        self.db
            .update_market_status(market_id, MarketStatus::Closed)
            .await
    }

    /// Delete a market and all associated data (admin only)
    pub async fn delete_market(&self, market_id: Uuid, admin_id: Uuid) -> DbResult<()> {
        let admin = self.db.get_user(admin_id).await?;

        if !admin.is_admin {
            return Err(crate::db::DbError::Constraint("Admin only".to_string()));
        }

        self.db.delete_market(market_id).await
    }

    /// Resolve a market (all bets resolved, final state)
    #[allow(dead_code)]
    pub async fn resolve_market(&self, market_id: Uuid, admin_id: Uuid) -> DbResult<()> {
        let admin = self.db.get_user(admin_id).await?;

        if !admin.is_admin {
            return Err(crate::db::DbError::Constraint("Admin only".to_string()));
        }

        self.db
            .update_market_status(market_id, MarketStatus::Resolved)
            .await
    }

    /// Get a market by ID
    pub async fn get_market(&self, market_id: Uuid) -> DbResult<Market> {
        self.db.get_market(market_id).await
    }

    /// Get a user by ID
    #[allow(dead_code)]
    pub async fn get_user(&self, user_id: Uuid) -> DbResult<User> {
        self.db.get_user(user_id).await
    }

    /// Get a bet by ID
    pub async fn get_bet(&self, bet_id: Uuid) -> DbResult<Bet> {
        self.db.get_bet(bet_id).await
    }

    /// Get all markets a device has joined (for recent markets feature)
    pub async fn get_markets_by_device_id(&self, device_id: &str) -> DbResult<Vec<(Market, User)>> {
        self.db.get_markets_by_device_id(device_id).await
    }
}

/// Generate a random 6-character invite code
fn generate_invite_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No confusing chars
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
