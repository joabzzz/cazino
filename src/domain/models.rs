use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Market status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketStatus {
    Draft,    // Collecting players and bet ideas
    Open,     // Active betting period
    Closed,   // Resolution period
    Resolved, // Final results
}

/// A prediction market (e.g., "Thanksgiving 2024")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: Uuid,
    pub name: String,
    pub status: MarketStatus,
    pub created_by: Uuid, // User ID of admin
    pub opens_at: DateTime<Utc>,
    pub closes_at: DateTime<Utc>,
    pub starting_balance: i64, // Default: 1000 coins
    pub invite_code: String,   // Short code for joining
    pub created_at: DateTime<Utc>,
}

/// A user in a market (Jackbox-style: device ID + display name)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub market_id: Uuid,
    pub device_id: String, // Generated on first visit, stored in cookie
    pub display_name: String,
    pub avatar: String, // Emoji
    pub balance: i64,   // Current coin balance
    pub is_admin: bool,
    pub joined_at: DateTime<Utc>,
}

/// Bet status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BetStatus {
    Pending,     // Awaiting admin approval
    Active,      // Open for betting
    ResolvedYes, // Outcome: YES
    ResolvedNo,  // Outcome: NO
    Challenged,  // Under dispute
}

/// Challenge status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum ChallengeStatus {
    Active,    // Challenge is ongoing
    Accepted,  // Resolver matched/raised
    Withdrawn, // Resolver withdrew
    Resolved,  // Dispute settled
}

/// A prediction bet about a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bet {
    pub id: Uuid,
    pub market_id: Uuid,
    pub subject_user_id: Uuid, // Who the bet is about
    pub created_by: Uuid,      // User ID who created it
    pub description: String,   // "Dad falls asleep during movie"
    pub initial_odds: String,  // e.g., "3:1" - for display only
    pub status: BetStatus,
    pub yes_pool: i64,           // Total coins bet on YES
    pub no_pool: i64,            // Total coins bet on NO
    pub hide_from_subject: bool, // If true, subject can't see this bet until resolved
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Betting side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    Yes,
    No,
}

/// A wager on a bet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wager {
    pub id: Uuid,
    pub bet_id: Uuid,
    pub user_id: Uuid,
    pub side: Side,
    pub amount: i64,
    pub placed_at: DateTime<Utc>,

    // Snapshot state after this wager (enables chart reconstruction)
    pub yes_pool_after: i64,
    pub no_pool_after: i64,
    pub probability_after: f64, // YES probability as decimal (0.0-1.0)
}

/// A challenge to a bet resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Challenge {
    pub id: Uuid,
    pub bet_id: Uuid,
    pub challenger_id: Uuid,   // Who initiated the challenge
    pub resolver_id: Uuid,     // Who resolved the bet (being challenged)
    pub challenger_stake: i64, // Current challenger stake
    pub resolver_stake: i64,   // Current resolver stake (must match)
    pub status: ChallengeStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub winner_id: Option<Uuid>, // Set when resolved
}

/// View model: Bet with visibility filtering applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetView {
    pub id: Uuid,
    pub market_id: Uuid,

    // Hidden if this bet is about the viewing user
    pub is_hidden: bool,

    // If hidden, these fields are redacted
    pub subject_user_id: Option<Uuid>,
    pub description: Option<String>,

    // Always visible
    pub created_by: Uuid,
    pub initial_odds: String,
    pub status: BetStatus,
    pub yes_pool: i64,
    pub no_pool: i64,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Bet {
    /// Convert to a view model for a specific viewing user
    pub fn to_view(&self, viewing_user_id: Uuid) -> BetView {
        // Hide from subject only if:
        // 1. The bet is marked as hide_from_subject by creator
        // 2. The viewing user IS the subject
        // 3. The bet is not yet resolved (reveal on resolution)
        let is_hidden = self.hide_from_subject
            && self.subject_user_id == viewing_user_id
            && self.status != BetStatus::ResolvedYes
            && self.status != BetStatus::ResolvedNo
            && self.status != BetStatus::Challenged;

        BetView {
            id: self.id,
            market_id: self.market_id,
            is_hidden,
            subject_user_id: if is_hidden {
                None
            } else {
                Some(self.subject_user_id)
            },
            description: if is_hidden {
                None
            } else {
                Some(self.description.clone())
            },
            created_by: self.created_by,
            initial_odds: self.initial_odds.clone(),
            status: self.status,
            yes_pool: self.yes_pool,
            no_pool: self.no_pool,
            created_at: self.created_at,
            resolved_at: self.resolved_at,
        }
    }
}

/// Probability chart data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityPoint {
    pub timestamp: DateTime<Utc>,
    pub yes_probability: f64,
}
