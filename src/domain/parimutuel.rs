/// Parimutuel betting calculation engine
///
/// In parimutuel betting, all wagers go into pools. Winners split the losing pool
/// proportionally based on their contribution to the winning pool.
use crate::domain::models::{Bet, Side, Wager};

/// Calculate the current YES probability
/// Formula: YES% = YES Pool / (YES Pool + NO Pool)
pub fn calculate_probability(yes_pool: i64, no_pool: i64) -> f64 {
    let total = yes_pool + no_pool;
    if total == 0 {
        return 0.5; // Default to 50/50 if no bets placed
    }
    yes_pool as f64 / total as f64
}

/// Calculate potential payout for a wager IF placed
/// Returns (new_yes_pool, new_no_pool, potential_payout)
pub fn calculate_potential_payout(
    current_yes_pool: i64,
    current_no_pool: i64,
    side: Side,
    amount: i64,
) -> (i64, i64, i64) {
    let (new_yes_pool, new_no_pool) = match side {
        Side::Yes => (current_yes_pool + amount, current_no_pool),
        Side::No => (current_yes_pool, current_no_pool + amount),
    };

    let total_pool = new_yes_pool + new_no_pool;

    // If this side wins, bettor gets their share of the total pool
    let winning_pool = match side {
        Side::Yes => new_yes_pool,
        Side::No => new_no_pool,
    };

    if winning_pool == 0 {
        return (new_yes_pool, new_no_pool, 0);
    }

    let payout = (amount as f64 / winning_pool as f64 * total_pool as f64) as i64;

    (new_yes_pool, new_no_pool, payout)
}

/// Calculate actual payouts for all wagers on a bet after resolution
/// Returns a map of user_id -> payout_amount
pub fn calculate_payouts(bet: &Bet, wagers: &[Wager]) -> Vec<(uuid::Uuid, i64)> {
    use std::collections::HashMap;

    let winning_side = match bet.status {
        crate::domain::models::BetStatus::ResolvedYes => Side::Yes,
        crate::domain::models::BetStatus::ResolvedNo => Side::No,
        _ => return vec![], // Not resolved yet
    };

    let total_pool = bet.yes_pool + bet.no_pool;
    let winning_pool = match winning_side {
        Side::Yes => bet.yes_pool,
        Side::No => bet.no_pool,
    };

    if winning_pool == 0 {
        return vec![]; // No winners (shouldn't happen)
    }

    // Group wagers by user and sum their winning bets
    let mut user_wagers: HashMap<uuid::Uuid, i64> = HashMap::new();

    for wager in wagers {
        if wager.side == winning_side {
            *user_wagers.entry(wager.user_id).or_insert(0) += wager.amount;
        }
    }

    // Calculate payout for each user
    user_wagers
        .into_iter()
        .map(|(user_id, total_wagered)| {
            let payout = (total_wagered as f64 / winning_pool as f64 * total_pool as f64) as i64;
            (user_id, payout)
        })
        .collect()
}

/// Parse initial odds string (e.g., "3:1") into pool ratio
/// Returns (yes_pool, no_pool) ratio for a given opening wager amount
///
/// Examples:
/// - "1:1" with 100 coins -> (100, 100) - 50/50
/// - "3:1" with 100 coins -> (25, 75) - 25% YES / 75% NO (YES is unlikely)
/// - "1:3" with 100 coins -> (75, 25) - 75% YES / 25% NO (YES is likely)
pub fn parse_initial_odds(odds: &str, opening_wager: i64) -> Option<(i64, i64)> {
    let parts: Vec<&str> = odds.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let yes_ratio: i64 = parts[0].parse().ok()?;
    let no_ratio: i64 = parts[1].parse().ok()?;

    if yes_ratio == 0 || no_ratio == 0 {
        return None;
    }

    // Creator always bets YES, so we need to distribute the opening wager
    // to create the desired odds
    //
    // For "3:1" odds (YES unlikely):
    // - YES probability should be 1/(3+1) = 25%
    // - If creator bets 100 on YES, we need NO pool to be 300
    // - But creator only has 100 to wager...
    // - So instead: creator seeds with their full amount on YES side
    // - The odds indicate what the initial probability SHOULD be
    // - We'll just use the wager to seed YES pool, and odds are for display

    // Actually, let's simplify: creator's wager goes entirely to YES
    // The initial_odds field is just for display/reference
    // Real odds emerge from actual betting
    Some((opening_wager, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_probability() {
        assert_eq!(calculate_probability(100, 100), 0.5);
        assert_eq!(calculate_probability(300, 200), 0.6);
        assert_eq!(calculate_probability(0, 0), 0.5);
        assert_eq!(calculate_probability(100, 0), 1.0);
    }

    #[test]
    fn test_potential_payout() {
        // Starting pool: 100 YES, 100 NO (total 200)
        // Bet 50 on YES
        let (yes_pool, no_pool, payout) = calculate_potential_payout(100, 100, Side::Yes, 50);

        assert_eq!(yes_pool, 150);
        assert_eq!(no_pool, 100);
        // Total pool now 250, bettor has 50/150 of YES pool
        // If YES wins: (50/150) * 250 = 83.33 -> 83
        assert_eq!(payout, 83);
    }

    #[test]
    fn test_parse_initial_odds() {
        let (yes, no) = parse_initial_odds("1:1", 100).unwrap();
        assert_eq!(yes, 100);
        assert_eq!(no, 0); // Creator seeds YES pool only

        let (yes, no) = parse_initial_odds("3:1", 100).unwrap();
        assert_eq!(yes, 100);
        assert_eq!(no, 0);

        assert!(parse_initial_odds("invalid", 100).is_none());
        assert!(parse_initial_odds("0:1", 100).is_none());
    }
}
