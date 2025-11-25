/// API request/response models
use crate::domain::models::{BetStatus, BetView, MarketStatus, Side};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== Request Models =====

#[derive(Debug, Deserialize)]
pub struct CreateMarketRequest {
    pub name: String,
    pub admin_name: String,
    pub duration_hours: i64,
    #[serde(default = "default_starting_balance")]
    pub starting_balance: i64,
    pub device_id: Option<String>,
    pub invite_code: Option<String>,
}

fn default_starting_balance() -> i64 {
    1000
}

#[derive(Debug, Deserialize)]
pub struct JoinMarketRequest {
    pub display_name: String,
    pub avatar: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBetRequest {
    pub subject_user_id: Uuid,
    pub description: String,
    pub initial_odds: String,
    pub opening_wager: i64,
    #[serde(default)]
    pub hide_from_subject: bool,
}

#[derive(Debug, Deserialize)]
pub struct PlaceWagerRequest {
    pub side: Side,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ResolveBetRequest {
    pub outcome: Side,
}

// ===== Response Models =====

#[derive(Debug, Serialize)]
pub struct CreateMarketResponse {
    pub market: crate::domain::models::Market,
    pub user: crate::domain::models::User,
    pub invite_code: String,
}

#[derive(Debug, Serialize)]
pub struct JoinMarketResponse {
    pub market: crate::domain::models::Market,
    pub user: crate::domain::models::User,
}

#[derive(Debug, Serialize)]
pub struct BetResponse {
    pub bet: BetView,
}

#[derive(Debug, Serialize)]
pub struct WagerResponse {
    pub bet_id: Uuid,
    pub user_id: Uuid,
    pub side: Side,
    pub amount: i64,
    pub new_probability: f64,
}

#[derive(Debug, Serialize)]
pub struct ProbabilityChartResponse {
    pub points: Vec<ProbabilityPoint>,
}

#[derive(Debug, Serialize)]
pub struct ProbabilityPoint {
    pub timestamp: String,
    pub yes_probability: f64,
}

#[derive(Debug, Serialize)]
pub struct LeaderboardResponse {
    pub users: Vec<UserWithStats>,
}

#[derive(Debug, Serialize)]
pub struct UserWithStats {
    pub user: crate::domain::models::User,
    pub profit: i64,
    pub rank: usize,
}

#[derive(Debug, Serialize)]
pub struct RevealResponse {
    pub bets: Vec<BetView>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceMarketsResponse {
    pub markets: Vec<DeviceMarketInfo>,
}

#[derive(Debug, Serialize)]
pub struct DeviceMarketInfo {
    pub market: crate::domain::models::Market,
    pub user: crate::domain::models::User,
}

// ===== WebSocket Messages =====

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    // Client -> Server
    #[serde(rename = "subscribe")]
    Subscribe { market_id: Uuid },

    #[serde(rename = "ping")]
    Ping,

    // Server -> Client
    #[serde(rename = "market_update")]
    MarketUpdate {
        market: crate::domain::models::Market,
    },

    #[serde(rename = "bet_created")]
    BetCreated { bet_id: Uuid, description: String },

    #[serde(rename = "bet_approved")]
    BetApproved { bet_id: Uuid },

    #[serde(rename = "user_joined")]
    UserJoined {
        user_id: Uuid,
        display_name: String,
        market_id: Uuid,
    },

    #[serde(rename = "wager_placed")]
    WagerPlaced {
        bet_id: Uuid,
        user_id: Uuid,
        side: Side,
        amount: i64,
        new_yes_pool: i64,
        new_no_pool: i64,
        new_probability: f64,
    },

    #[serde(rename = "bet_resolved")]
    BetResolved {
        bet_id: Uuid,
        outcome: Side,
        status: BetStatus,
    },

    #[serde(rename = "market_status_changed")]
    MarketStatusChanged {
        market_id: Uuid,
        status: MarketStatus,
    },

    #[serde(rename = "market_deleted")]
    MarketDeleted { market_id: Uuid },

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "error")]
    Error { message: String },
}
