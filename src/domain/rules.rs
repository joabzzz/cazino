/// Game rules and validation logic
use crate::domain::models::{Bet, BetStatus, Market, MarketStatus, User};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RuleError {
    #[error("Market is not open for betting")]
    MarketNotOpen,

    #[error("Bet is not active")]
    BetNotActive,

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: i64, available: i64 },

    #[error("Cannot bet on bets about yourself")]
    CannotBetOnSelf,

    #[error("Only admin can perform this action")]
    AdminOnly,

    #[error("Invalid wager amount: {0}")]
    InvalidAmount(String),

    #[error("Bet already resolved")]
    AlreadyResolved,

    #[error("Market not in correct status for this action")]
    InvalidMarketStatus,
}

/// Validate that a user can place a wager
pub fn validate_wager(
    market: &Market,
    bet: &Bet,
    user: &User,
    amount: i64,
) -> Result<(), RuleError> {
    // Market must be open
    if market.status != MarketStatus::Open {
        return Err(RuleError::MarketNotOpen);
    }

    // Bet must be active
    if bet.status != BetStatus::Active {
        return Err(RuleError::BetNotActive);
    }

    // User must have sufficient balance
    if user.balance < amount {
        return Err(RuleError::InsufficientBalance {
            needed: amount,
            available: user.balance,
        });
    }

    // Amount must be positive
    if amount <= 0 {
        return Err(RuleError::InvalidAmount(
            "Amount must be positive".to_string(),
        ));
    }

    // CRITICAL: Cannot bet on bets about yourself
    // (This is the core "hidden bet" mechanic)
    if bet.subject_user_id == user.id {
        return Err(RuleError::CannotBetOnSelf);
    }

    Ok(())
}

/// Validate that a user can create a bet
pub fn validate_bet_creation(
    market: &Market,
    user: &User,
    _subject_user_id: Uuid,
    opening_wager: i64,
) -> Result<(), RuleError> {
    // Market must be in draft or open status
    if market.status != MarketStatus::Draft && market.status != MarketStatus::Open {
        return Err(RuleError::InvalidMarketStatus);
    }

    // User must have sufficient balance for opening wager
    if user.balance < opening_wager {
        return Err(RuleError::InsufficientBalance {
            needed: opening_wager,
            available: user.balance,
        });
    }

    // Opening wager must be positive
    if opening_wager <= 0 {
        return Err(RuleError::InvalidAmount(
            "Opening wager must be positive".to_string(),
        ));
    }

    // Subject must be in the same market
    // (This would be validated at the DB layer, but we document it here)

    Ok(())
}

/// Validate that a user can approve/reject a bet (admin only)
pub fn validate_bet_approval(user: &User) -> Result<(), RuleError> {
    if !user.is_admin {
        return Err(RuleError::AdminOnly);
    }
    Ok(())
}

/// Validate that a bet can be resolved
pub fn validate_bet_resolution(_market: &Market, bet: &Bet, user: &User) -> Result<(), RuleError> {
    // Only admin can resolve
    if !user.is_admin {
        return Err(RuleError::AdminOnly);
    }

    // Market should be closed (but we allow resolution in open for MVP flexibility)
    // if market.status != MarketStatus::Closed {
    //     return Err(RuleError::InvalidMarketStatus);
    // }

    // Bet must be active
    if bet.status != BetStatus::Active {
        if bet.status == BetStatus::ResolvedYes || bet.status == BetStatus::ResolvedNo {
            return Err(RuleError::AlreadyResolved);
        }
        return Err(RuleError::BetNotActive);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mock_market() -> Market {
        Market {
            id: Uuid::new_v4(),
            name: "Test Market".to_string(),
            status: MarketStatus::Open,
            created_by: Uuid::new_v4(),
            opens_at: Utc::now(),
            closes_at: Utc::now(),
            starting_balance: 1000,
            invite_code: "TEST".to_string(),
            created_at: Utc::now(),
        }
    }

    fn mock_user(balance: i64, is_admin: bool) -> User {
        User {
            id: Uuid::new_v4(),
            market_id: Uuid::new_v4(),
            device_id: "test-device".to_string(),
            display_name: "Test User".to_string(),
            avatar: "🎲".to_string(),
            balance,
            is_admin,
            joined_at: Utc::now(),
        }
    }

    fn mock_bet(subject_id: Uuid) -> Bet {
        Bet {
            id: Uuid::new_v4(),
            market_id: Uuid::new_v4(),
            subject_user_id: subject_id,
            created_by: Uuid::new_v4(),
            description: "Test bet".to_string(),
            resolution_criteria: "Test criteria".to_string(),
            initial_odds: "1:1".to_string(),
            status: BetStatus::Active,
            yes_pool: 0,
            no_pool: 0,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    #[test]
    fn test_validate_wager_success() {
        let market = mock_market();
        let user = mock_user(1000, false);
        let bet = mock_bet(Uuid::new_v4()); // Different user

        assert!(validate_wager(&market, &bet, &user, 100).is_ok());
    }

    #[test]
    fn test_validate_wager_insufficient_balance() {
        let market = mock_market();
        let user = mock_user(50, false);
        let bet = mock_bet(Uuid::new_v4());

        let result = validate_wager(&market, &bet, &user, 100);
        assert!(matches!(result, Err(RuleError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_validate_wager_cannot_bet_on_self() {
        let market = mock_market();
        let user = mock_user(1000, false);
        let bet = mock_bet(user.id); // Bet about this user

        let result = validate_wager(&market, &bet, &user, 100);
        assert!(matches!(result, Err(RuleError::CannotBetOnSelf)));
    }

    #[test]
    fn test_validate_wager_market_not_open() {
        let mut market = mock_market();
        market.status = MarketStatus::Closed;
        let user = mock_user(1000, false);
        let bet = mock_bet(Uuid::new_v4());

        let result = validate_wager(&market, &bet, &user, 100);
        assert!(matches!(result, Err(RuleError::MarketNotOpen)));
    }
}
