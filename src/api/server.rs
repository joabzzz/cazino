/// HTTP + WebSocket server
use crate::api::routes::{self, AppState};
use crate::api::websocket;
use crate::db::Database;
use crate::service::CazinoService;
use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// Run the HTTP + WebSocket server
pub async fn run_server<D>(service: CazinoService<D>, port: u16) -> anyhow::Result<()>
where
    D: Database + Clone + Send + Sync + 'static,
{
    // Create broadcast channel for WebSocket messages
    let (broadcast_tx, _) = websocket::create_broadcast_channel();

    let state = AppState {
        service: Arc::new(service),
        broadcast_tx: Arc::new(broadcast_tx),
    };

    let app = create_router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("\n🎲 Cazino API Server Started!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📡 HTTP API:    http://localhost:{}", port);
    println!("🔌 WebSocket:   ws://localhost:{}/ws", port);
    println!("❤️  Health:      http://localhost:{}/health", port);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📚 API Docs:    See API.md");
    println!("🧪 Test:        cargo run -- cli");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    tracing::info!("🎲 Cazino API server listening on {}", addr);
    tracing::info!("📡 WebSocket endpoint: ws://{}/ws", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router<D: Database + Clone + Send + Sync + 'static>(state: AppState<D>) -> Router {
    Router::new()
        // Serve static UI files
        .nest_service("/", ServeDir::new("ui"))
        // WebSocket endpoint (with market_id parameter)
        .route("/ws/:market_id", get(ws_handler))
        // Market routes
        .route("/api/markets", post(routes::create_market::<D>))
        .route("/api/markets/:market_id", get(routes::get_market::<D>))
        .route(
            "/api/markets/:invite_code/join",
            post(routes::join_market::<D>),
        )
        .route(
            "/api/markets/:market_id/leaderboard",
            get(routes::get_leaderboard::<D>),
        )
        .route(
            "/api/markets/:market_id/open/:admin_id",
            post(routes::open_market::<D>),
        )
        .route(
            "/api/markets/:market_id/close/:admin_id",
            post(routes::close_market::<D>),
        )
        .route(
            "/api/markets/:market_id/delete/:admin_id",
            post(routes::delete_market::<D>),
        )
        // Bet routes
        .route(
            "/api/markets/:market_id/bets/:user_id",
            get(routes::get_bets::<D>),
        )
        .route(
            "/api/markets/:market_id/bets/:creator_id/create",
            post(routes::create_bet::<D>),
        )
        .route(
            "/api/markets/:market_id/bets/pending",
            get(routes::get_pending_bets::<D>),
        )
        .route(
            "/api/bets/:bet_id/approve/:admin_id",
            post(routes::approve_bet::<D>),
        )
        .route(
            "/api/bets/:bet_id/wager/:user_id",
            post(routes::place_wager::<D>),
        )
        .route(
            "/api/bets/:bet_id/chart",
            get(routes::get_probability_chart::<D>),
        )
        .route(
            "/api/bets/:bet_id/resolve/:admin_id",
            post(routes::resolve_bet::<D>),
        )
        // Reveal route
        .route("/api/users/:user_id/reveal", get(routes::get_reveal::<D>))
        // Device routes (fingerprint-based)
        .route(
            "/api/devices/:device_id/markets",
            get(routes::get_device_markets::<D>),
        )
        // Health check
        .route("/health", get(health_check))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}

async fn ws_handler<D: Database + Clone + Send + Sync + 'static>(
    ws: WebSocketUpgrade,
    Path(market_id): Path<String>,
    State(state): State<AppState<D>>,
) -> impl IntoResponse {
    tracing::info!("🔌 WebSocket upgrade requested for market: {}", market_id);
    ws.on_upgrade(move |socket| websocket::handle_socket(socket, state.broadcast_tx))
}

async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SqliteDatabase;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_router_creation() {
        let db = SqliteDatabase::new("sqlite::memory:").await.unwrap();
        db.run_migrations().await.unwrap();
        let service = CazinoService::new(Arc::new(db));

        let (broadcast_tx, _) = websocket::create_broadcast_channel();

        let state = AppState {
            service: Arc::new(service),
            broadcast_tx: Arc::new(broadcast_tx),
        };

        let _router = create_router(state);
        // Just ensure the router can be created
    }
}
