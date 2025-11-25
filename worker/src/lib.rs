/// Cloudflare Worker for Cazino API
use worker::*;

mod d1_database;
mod room;

use cazino::api::models::*;
use cazino::domain::models::BetView;
use cazino::service::CazinoService;
use d1_database::D1Database;
use std::sync::Arc;
use uuid::Uuid;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // Handle CORS preflight requests
    if req.method() == Method::Options {
        let mut response = Response::empty()?;
        response
            .headers_mut()
            .set("Access-Control-Allow-Origin", "*")?;
        response.headers_mut().set(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        )?;
        response.headers_mut().set(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        )?;
        response
            .headers_mut()
            .set("Access-Control-Max-Age", "86400")?;
        return Ok(response);
    }

    // Get D1 database from environment
    let d1 = env.d1("CAZINO_DB")?;
    let db = Arc::new(D1Database::new(d1));
    let service = Arc::new(CazinoService::new(db));

    // Create router - clone service for each route
    let router = Router::new();

    let svc1 = service.clone();
    let svc2 = service.clone();
    let svc3 = service.clone();
    let svc4 = service.clone();
    let svc5 = service.clone();
    let svc6 = service.clone();
    let svc7 = service.clone();
    let svc8 = service.clone();
    let svc9 = service.clone();
    let svc10 = service.clone();
    let svc11 = service.clone();
    let svc12 = service.clone();
    let svc13 = service.clone();
    let svc14 = service.clone();
    let svc15 = service.clone();

    router
        // Market routes
        .post_async("/api/markets", move |req, _ctx| {
            let service = svc1.clone();
            async move { handle_create_market(req, service).await }
        })
        .post_async("/api/markets/:invite_code/join", move |req, ctx| {
            let service = svc2.clone();
            async move { handle_join_market(req, ctx, service).await }
        })
        .get_async("/api/markets/:market_id", move |_req, ctx| {
            let service = svc3.clone();
            async move { handle_get_market(ctx, service).await }
        })
        .get_async("/api/markets/:market_id/leaderboard", move |_req, ctx| {
            let service = svc4.clone();
            async move { handle_get_leaderboard(ctx, service).await }
        })
        .post_async(
            "/api/markets/:market_id/open/:admin_id",
            move |_req, ctx| {
                let service = svc5.clone();
                async move { handle_open_market(ctx, service).await }
            },
        )
        .post_async(
            "/api/markets/:market_id/close/:admin_id",
            move |_req, ctx| {
                let service = svc6.clone();
                async move { handle_close_market(ctx, service).await }
            },
        )
        // Bet routes
        .get_async("/api/markets/:market_id/bets/:user_id", move |_req, ctx| {
            let service = svc7.clone();
            async move { handle_get_bets(ctx, service).await }
        })
        .post_async(
            "/api/markets/:market_id/bets/:creator_id",
            move |req, ctx| {
                let service = svc8.clone();
                async move { handle_create_bet(req, ctx, service).await }
            },
        )
        .get_async("/api/markets/:market_id/bets-pending", move |_req, ctx| {
            let service = svc9.clone();
            async move { handle_get_pending_bets(ctx, service).await }
        })
        .post_async("/api/bets/:bet_id/approve/:admin_id", move |_req, ctx| {
            let service = svc10.clone();
            async move { handle_approve_bet(ctx, service).await }
        })
        .post_async("/api/bets/:bet_id/wager/:user_id", move |req, ctx| {
            let service = svc11.clone();
            async move { handle_place_wager(req, ctx, service).await }
        })
        .get_async("/api/bets/:bet_id/chart", move |_req, ctx| {
            let service = svc12.clone();
            async move { handle_get_probability_chart(ctx, service).await }
        })
        .post_async("/api/bets/:bet_id/resolve/:admin_id", move |req, ctx| {
            let service = svc13.clone();
            async move { handle_resolve_bet(req, ctx, service).await }
        })
        // User routes
        .get_async("/api/users/:user_id/reveal", move |_req, ctx| {
            let service = svc14.clone();
            async move { handle_get_reveal(ctx, service).await }
        })
        // Device routes (fingerprint-based)
        .get_async("/api/devices/:device_id/markets", move |_req, ctx| {
            let service = svc15.clone();
            async move { handle_get_device_markets(ctx, service).await }
        })
        // WebSocket route - forward to Durable Object
        .get_async("/ws/:market_id", |req, ctx| async move {
            handle_websocket(req, ctx).await
        })
        .run(req, env)
        .await
}

// ===== Helper Functions =====

/// Broadcast a message to all WebSocket clients connected to a market
async fn broadcast_to_market(
    ctx: &RouteContext<()>,
    market_id: &str,
    message: serde_json::Value,
) -> Result<()> {
    console_log!("Broadcasting to market {}: {:?}", market_id, message);

    // Get the Durable Object namespace
    let namespace = ctx.durable_object("ROOM")?;

    // Create a Durable Object ID for this market
    let id = namespace.id_from_name(market_id)?;

    // Get the Durable Object stub
    let stub = id.get_stub()?;

    // Serialize the message
    let body = serde_json::to_string(&message)
        .map_err(|e| Error::RustError(format!("Failed to serialize message: {}", e)))?;

    // Create a POST request to the /broadcast endpoint
    let mut request = Request::new_with_init(
        "https://fake-host/broadcast",
        RequestInit::new()
            .with_method(Method::Post)
            .with_body(Some(body.into())),
    )?;

    request
        .headers_mut()?
        .set("Content-Type", "application/json")?;

    // Send the request to the Durable Object
    let response = stub.fetch_with_request(request).await?;

    if response.status_code() != 200 {
        console_log!("Broadcast failed with status: {}", response.status_code());
    }

    Ok(())
}

// ===== Handler Functions =====

async fn handle_create_market(
    mut req: Request,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let body: CreateMarketRequest = req.json().await?;

    let device_id = body
        .device_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let (market, user) = service
        .create_market(
            body.name,
            device_id,
            body.admin_name,
            "👑".to_string(),
            body.starting_balance,
            body.duration_hours,
            body.invite_code,
        )
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let invite_code = market.invite_code.clone();

    let response = CreateMarketResponse {
        market,
        user,
        invite_code,
    };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_join_market(
    mut req: Request,
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let invite_code = ctx.param("invite_code").unwrap().to_string();
    let body: JoinMarketRequest = req.json().await?;

    let device_id = body
        .device_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let (market, user) = service
        .join_market(invite_code, device_id, body.display_name, body.avatar)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast user joined event to all connected clients
    let broadcast_msg = serde_json::json!({
        "type": "user_joined",
        "data": {
            "user_id": user.id,
            "display_name": user.display_name,
            "avatar": user.avatar,
            "balance": user.balance
        }
    });

    let _ = broadcast_to_market(&ctx, &market.id.to_string(), broadcast_msg).await;

    let response = JoinMarketResponse { market, user };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_get_market(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;

    let market = service
        .get_market(market_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    Response::from_json(&market).and_then(|r| add_cors_headers(r))
}

async fn handle_get_leaderboard(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;

    let market = service
        .get_market(market_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let mut users = service
        .get_users(market_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

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

    let response = LeaderboardResponse {
        users: users_with_stats,
    };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_open_market(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;
    let admin_id = parse_uuid(ctx.param("admin_id").unwrap())?;

    service
        .open_market(market_id, admin_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Get updated market to broadcast
    let market = service
        .get_market(market_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast market opened event to all connected clients
    let broadcast_msg = serde_json::json!({
        "type": "market_opened",
        "data": {
            "market_id": market_id,
            "status": format!("{:?}", market.status)
        }
    });

    broadcast_to_market(&ctx, &market_id.to_string(), broadcast_msg).await?;

    Response::empty().and_then(|r| add_cors_headers(r))
}

async fn handle_close_market(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;
    let admin_id = parse_uuid(ctx.param("admin_id").unwrap())?;

    service
        .close_market(market_id, admin_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    Response::empty().and_then(|r| add_cors_headers(r))
}

async fn handle_get_bets(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;
    let user_id = parse_uuid(ctx.param("user_id").unwrap())?;

    let bets = service
        .get_bets(market_id, user_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    Response::from_json(&bets).and_then(|r| add_cors_headers(r))
}

async fn handle_create_bet(
    mut req: Request,
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;
    let creator_id = parse_uuid(ctx.param("creator_id").unwrap())?;
    let body: CreateBetRequest = req.json().await?;

    let bet = service
        .create_bet(
            market_id,
            creator_id,
            body.subject_user_id,
            body.description,
            body.initial_odds,
            body.opening_wager,
        )
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast bet created event to all connected clients
    let yes_prob = cazino::domain::parimutuel::calculate_probability(bet.yes_pool, bet.no_pool);
    let broadcast_msg = serde_json::json!({
        "type": "bet_created",
        "data": {
            "bet_id": bet.id,
            "description": bet.description,
            "created_by": bet.created_by,
            "subject_user_id": bet.subject_user_id,
            "status": format!("{:?}", bet.status),
            "yes_probability": yes_prob
        }
    });

    let _ = broadcast_to_market(&ctx, &market_id.to_string(), broadcast_msg).await;

    let response = BetResponse {
        bet: bet.to_view(creator_id),
    };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_get_pending_bets(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let market_id = parse_uuid(ctx.param("market_id").unwrap())?;

    let bets = service
        .get_pending_bets(market_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Convert to BetView (admin can see all)
    let bet_views: Vec<BetView> = bets.iter().map(|b| b.to_view(Uuid::nil())).collect();

    Response::from_json(&bet_views).and_then(|r| add_cors_headers(r))
}

async fn handle_approve_bet(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let bet_id = parse_uuid(ctx.param("bet_id").unwrap())?;
    let admin_id = parse_uuid(ctx.param("admin_id").unwrap())?;

    // Get bet before approving to get market_id
    let bet = service
        .get_bet(bet_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let market_id = bet.market_id;

    service
        .approve_bet(bet_id, admin_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast bet approved event to all connected clients
    let broadcast_msg = serde_json::json!({
        "type": "bet_approved",
        "data": {
            "bet_id": bet_id,
            "description": bet.description
        }
    });

    let _ = broadcast_to_market(&ctx, &market_id.to_string(), broadcast_msg).await;

    Response::empty().and_then(|r| add_cors_headers(r))
}

async fn handle_place_wager(
    mut req: Request,
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let bet_id = parse_uuid(ctx.param("bet_id").unwrap())?;
    let user_id = parse_uuid(ctx.param("user_id").unwrap())?;
    let body: PlaceWagerRequest = req.json().await?;

    let wager = service
        .place_wager(bet_id, user_id, body.side, body.amount)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Get bet to find market_id for broadcasting
    let bet = service
        .get_bet(bet_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast wager placed event to all connected clients
    let broadcast_msg = serde_json::json!({
        "type": "wager_placed",
        "data": {
            "bet_id": bet_id,
            "user_id": user_id,
            "side": format!("{:?}", body.side),
            "amount": body.amount,
            "new_probability": wager.probability_after
        }
    });

    let _ = broadcast_to_market(&ctx, &bet.market_id.to_string(), broadcast_msg).await;

    let response = WagerResponse {
        bet_id,
        user_id,
        side: body.side,
        amount: body.amount,
        new_probability: wager.probability_after,
    };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_get_probability_chart(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let bet_id = parse_uuid(ctx.param("bet_id").unwrap())?;

    let chart = service
        .get_probability_chart(bet_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let points: Vec<ProbabilityPoint> = chart
        .into_iter()
        .map(|p| ProbabilityPoint {
            timestamp: p.timestamp.to_rfc3339(),
            yes_probability: p.yes_probability,
        })
        .collect();

    let response = ProbabilityChartResponse { points };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_resolve_bet(
    mut req: Request,
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let bet_id = parse_uuid(ctx.param("bet_id").unwrap())?;
    let admin_id = parse_uuid(ctx.param("admin_id").unwrap())?;
    let body: ResolveBetRequest = req.json().await?;

    // Get bet before resolving to get market_id
    let bet = service
        .get_bet(bet_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let market_id = bet.market_id;

    service
        .resolve_bet(bet_id, admin_id, body.outcome)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // Broadcast bet resolved event to all connected clients
    let broadcast_msg = serde_json::json!({
        "type": "bet_resolved",
        "data": {
            "bet_id": bet_id,
            "outcome": body.outcome
        }
    });

    let _ = broadcast_to_market(&ctx, &market_id.to_string(), broadcast_msg).await;

    Response::empty().and_then(|r| add_cors_headers(r))
}

async fn handle_get_reveal(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let user_id = parse_uuid(ctx.param("user_id").unwrap())?;

    let bets = service
        .get_bets_about_user(user_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    // For reveal, show full details (use nil UUID so nothing is hidden)
    let bet_views: Vec<BetView> = bets.iter().map(|b| b.to_view(Uuid::nil())).collect();

    let response = RevealResponse { bets: bet_views };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

async fn handle_get_device_markets(
    ctx: RouteContext<()>,
    service: Arc<CazinoService<D1Database>>,
) -> Result<Response> {
    let device_id = ctx.param("device_id").unwrap().to_string();

    console_log!("Getting markets for device: {}", device_id);

    let markets = service
        .get_markets_by_device_id(&device_id)
        .await
        .map_err(|e| Error::RustError(e.to_string()))?;

    let market_infos: Vec<DeviceMarketInfo> = markets
        .into_iter()
        .map(|(market, user)| DeviceMarketInfo { market, user })
        .collect();

    let response = DeviceMarketsResponse {
        markets: market_infos,
    };

    Response::from_json(&response).and_then(|r| add_cors_headers(r))
}

#[derive(Debug, serde::Serialize)]
struct DeviceMarketsResponse {
    markets: Vec<DeviceMarketInfo>,
}

#[derive(Debug, serde::Serialize)]
struct DeviceMarketInfo {
    market: cazino::domain::models::Market,
    user: cazino::domain::models::User,
}

// ===== Helper Functions =====

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| Error::RustError(format!("Invalid UUID: {}", e)))
}

fn add_cors_headers(mut response: Response) -> Result<Response> {
    let headers = response.headers_mut();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS",
    )?;
    headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    )?;
    Ok(response)
}

// ===== WebSocket Handler =====

async fn handle_websocket(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let market_id = ctx.param("market_id").map_or("default", |v| v.as_str());

    console_log!("WebSocket request for market: {}", market_id);

    // Get the Durable Object namespace
    let namespace = ctx.durable_object("ROOM")?;

    // Create a Durable Object ID for this market
    let id = namespace.id_from_name(market_id)?;

    // Get the Durable Object stub
    let stub = id.get_stub()?;

    // Forward the request to the Durable Object
    stub.fetch_with_request(req).await
}
