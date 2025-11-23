/// HTTP API routes
use crate::api::models::*;
use crate::api::websocket::{broadcast, BroadcastTx};
use crate::db::Database;
use crate::domain::models::{BetView, Market};
use crate::service::CazinoService;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

/// Shared application state
#[derive(Clone)]
pub struct AppState<D: Database> {
    pub service: Arc<CazinoService<D>>,
    pub broadcast_tx: Arc<BroadcastTx>,
}

// ===== Market Routes =====

/// Create a new market
pub async fn create_market<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Json(req): Json<CreateMarketRequest>,
) -> Result<Json<CreateMarketResponse>, ApiError> {
    tracing::info!("📊 Creating market: '{}'", req.name);

    // Use provided device_id or generate a new one
    let device_id = req.device_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let (market, user) = state
        .service
        .create_market(
            req.name.clone(),
            device_id,
            req.admin_name.clone(),
            "👑".to_string(),
            req.starting_balance,
            req.duration_hours,
        )
        .await?;

    let invite_code = market.invite_code.clone();

    tracing::info!(
        "✅ Market created: {} | Admin: {} | Invite: {}",
        market.id,
        user.display_name,
        invite_code
    );

    // Broadcast market creation
    broadcast(
        &state.broadcast_tx,
        WsMessage::MarketUpdate {
            market: market.clone(),
        },
    );

    Ok(Json(CreateMarketResponse {
        market,
        user,
        invite_code,
    }))
}

/// Join an existing market
pub async fn join_market<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(invite_code): Path<String>,
    Json(req): Json<JoinMarketRequest>,
) -> Result<Json<JoinMarketResponse>, ApiError> {
    tracing::info!(
        "👤 User '{}' joining market with code: {}",
        req.display_name,
        invite_code
    );

    // Use provided device_id or generate a new one
    let device_id = req.device_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let (market, user) = state
        .service
        .join_market(invite_code, device_id, req.display_name.clone(), req.avatar)
        .await?;

    tracing::info!(
        "✅ {} joined market: {} ({})",
        req.display_name,
        market.name,
        market.id
    );

    // Broadcast user joined
    broadcast(
        &state.broadcast_tx,
        WsMessage::UserJoined {
            user_id: user.id,
            display_name: user.display_name.clone(),
            market_id: market.id,
        },
    );

    Ok(Json(JoinMarketResponse { market, user }))
}

/// Get market details
pub async fn get_market<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<Market>, ApiError> {
    let market = state.service.get_market(market_id).await?;
    Ok(Json(market))
}

/// Get all users in a market (leaderboard)
pub async fn get_leaderboard<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<LeaderboardResponse>, ApiError> {
    let market = state.service.get_market(market_id).await?;
    let mut users = state.service.get_users(market_id).await?;

    // Sort by balance descending
    users.sort_by(|a, b| b.balance.cmp(&a.balance));

    let users_with_stats: Vec<UserWithStats> = users
        .into_iter()
        .enumerate()
        .map(|(idx, user)| UserWithStats {
            profit: user.balance - market.starting_balance,
            rank: idx + 1,
            user,
        })
        .collect();

    Ok(Json(LeaderboardResponse {
        users: users_with_stats,
    }))
}

/// Open market for betting (admin only)
pub async fn open_market<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((market_id, admin_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("🔓 Admin {} opening market {}", admin_id, market_id);

    state.service.open_market(market_id, admin_id).await?;

    let market = state.service.get_market(market_id).await?;

    tracing::info!("✅ Market '{}' is now OPEN for betting", market.name);

    broadcast(
        &state.broadcast_tx,
        WsMessage::MarketStatusChanged {
            market_id,
            status: market.status,
        },
    );

    Ok(StatusCode::OK)
}

/// Close market (admin only)
pub async fn close_market<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((market_id, admin_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("🔒 Admin {} closing market {}", admin_id, market_id);

    state.service.close_market(market_id, admin_id).await?;

    let market = state.service.get_market(market_id).await?;

    tracing::info!("✅ Market '{}' is now CLOSED", market.name);

    broadcast(
        &state.broadcast_tx,
        WsMessage::MarketStatusChanged {
            market_id,
            status: market.status,
        },
    );

    Ok(StatusCode::OK)
}

// ===== Bet Routes =====

/// Get all bets in a market (filtered for viewing user)
pub async fn get_bets<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((market_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<BetView>>, ApiError> {
    let bets = state.service.get_bets(market_id, user_id).await?;
    Ok(Json(bets))
}

/// Create a new bet
pub async fn create_bet<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((market_id, creator_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateBetRequest>,
) -> Result<Json<BetResponse>, ApiError> {
    tracing::info!(
        "🎲 Creating bet: '{}' | Opening wager: {}",
        req.description,
        req.opening_wager
    );

    let bet = state
        .service
        .create_bet(
            market_id,
            creator_id,
            req.subject_user_id,
            req.description.clone(),
            req.initial_odds,
            req.opening_wager,
        )
        .await?;

    tracing::info!(
        "✅ Bet created: {} | Status: pending | Pools: {} YES / {} NO",
        bet.id,
        bet.yes_pool,
        bet.no_pool
    );

    broadcast(
        &state.broadcast_tx,
        WsMessage::BetCreated {
            bet_id: bet.id,
            description: req.description,
        },
    );

    Ok(Json(BetResponse {
        bet: bet.to_view(creator_id),
    }))
}

/// Get pending bets (admin only)
pub async fn get_pending_bets<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<Vec<BetView>>, ApiError> {
    let bets = state.service.get_pending_bets(market_id).await?;
    // Convert to BetView (admin can see all)
    let bet_views: Vec<BetView> = bets.iter().map(|b| b.to_view(Uuid::nil())).collect();
    Ok(Json(bet_views))
}

/// Approve a bet (admin only)
pub async fn approve_bet<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((bet_id, admin_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("✅ Admin {} approving bet {}", admin_id, bet_id);

    state.service.approve_bet(bet_id, admin_id).await?;

    tracing::info!("✅ Bet {} is now ACTIVE and open for wagering", bet_id);

    broadcast(&state.broadcast_tx, WsMessage::BetApproved { bet_id });

    Ok(StatusCode::OK)
}

/// Place a wager on a bet
pub async fn place_wager<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((bet_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<PlaceWagerRequest>,
) -> Result<Json<WagerResponse>, ApiError> {
    tracing::info!(
        "💰 User {} wagering {} on {:?} for bet {}",
        user_id,
        req.amount,
        req.side,
        bet_id
    );

    let wager = state
        .service
        .place_wager(bet_id, user_id, req.side, req.amount)
        .await?;

    tracing::info!(
        "✅ Wager placed | Pools: {} YES / {} NO | Probability: {:.1}% YES",
        wager.yes_pool_after,
        wager.no_pool_after,
        wager.probability_after * 100.0
    );

    // Broadcast wager to all connected clients
    broadcast(
        &state.broadcast_tx,
        WsMessage::WagerPlaced {
            bet_id,
            user_id,
            side: req.side,
            amount: req.amount,
            new_yes_pool: wager.yes_pool_after,
            new_no_pool: wager.no_pool_after,
            new_probability: wager.probability_after,
        },
    );

    Ok(Json(WagerResponse {
        bet_id,
        user_id,
        side: req.side,
        amount: req.amount,
        new_probability: wager.probability_after,
    }))
}

/// Get probability chart for a bet
pub async fn get_probability_chart<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(bet_id): Path<Uuid>,
) -> Result<Json<ProbabilityChartResponse>, ApiError> {
    let chart = state.service.get_probability_chart(bet_id).await?;

    let points: Vec<ProbabilityPoint> = chart
        .into_iter()
        .map(|p| ProbabilityPoint {
            timestamp: p.timestamp.to_rfc3339(),
            yes_probability: p.yes_probability,
        })
        .collect();

    Ok(Json(ProbabilityChartResponse { points }))
}

/// Resolve a bet (admin only)
pub async fn resolve_bet<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path((bet_id, admin_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ResolveBetRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(
        "🏁 Admin {} resolving bet {} as {:?}",
        admin_id,
        bet_id,
        req.outcome
    );

    state
        .service
        .resolve_bet(bet_id, admin_id, req.outcome)
        .await?;

    let bet = state.service.get_bet(bet_id).await?;

    tracing::info!(
        "✅ Bet resolved | Outcome: {:?} | Total pool: {} coins distributed",
        req.outcome,
        bet.yes_pool + bet.no_pool
    );

    broadcast(
        &state.broadcast_tx,
        WsMessage::BetResolved {
            bet_id,
            outcome: req.outcome,
            status: bet.status,
        },
    );

    Ok(StatusCode::OK)
}

/// Get bets about a specific user (reveal screen)
pub async fn get_reveal<D: Database + 'static>(
    State(state): State<AppState<D>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<RevealResponse>, ApiError> {
    let bets = state.service.get_bets_about_user(user_id).await?;
    // For reveal, show full details (use nil UUID so nothing is hidden)
    let bet_views: Vec<BetView> = bets.iter().map(|b| b.to_view(Uuid::nil())).collect();

    Ok(Json(RevealResponse { bets: bet_views }))
}

// ===== Error Handling =====

pub struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let error_message = self.0.to_string();

        tracing::error!("❌ API Error: {}", error_message);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: error_message,
            }),
        )
            .into_response()
    }
}
