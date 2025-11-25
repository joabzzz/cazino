use serde::{Deserialize, Serialize};
/// Durable Object for WebSocket room management
/// Each market gets its own Durable Object instance
use worker::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    // Client -> Server
    #[serde(rename = "subscribe")]
    Subscribe { market_id: String },

    #[serde(rename = "ping")]
    Ping,

    // Server -> Client
    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "subscribed")]
    Subscribed { market_id: String },

    #[serde(rename = "market_update")]
    MarketUpdate { data: serde_json::Value },

    #[serde(rename = "bet_created")]
    BetCreated { bet_id: String, description: String },

    #[serde(rename = "bet_updated")]
    BetUpdated { bet_id: String },

    #[serde(rename = "wager_placed")]
    WagerPlaced { bet_id: String, amount: i64 },

    #[serde(rename = "bet_resolved")]
    BetResolved { bet_id: String, outcome: bool },
}

#[durable_object]
pub struct CazinoRoom {
    state: State,
    _env: Env,
}

impl DurableObject for CazinoRoom {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        // Check if this is a broadcast request (POST /broadcast)
        if req.method() == Method::Post && req.path().ends_with("/broadcast") {
            // Parse the JSON message
            let message: serde_json::Value = req.json().await?;
            let message_str = serde_json::to_string(&message)
                .map_err(|e| Error::RustError(format!("Failed to serialize message: {}", e)))?;

            console_log!("Broadcasting message: {}", message_str);

            // Broadcast to all connected clients
            self.broadcast(&message_str)?;

            return Response::ok("Broadcast sent");
        }

        // Check for WebSocket upgrade
        let upgrade_header = req.headers().get("Upgrade")?;

        if upgrade_header.as_deref() != Some("websocket") {
            return Response::error("Expected WebSocket upgrade", 426);
        }

        // Create WebSocket pair
        let pair = WebSocketPair::new()?;
        let server = pair.server;
        let client = pair.client;

        // Accept the WebSocket using hibernation API
        // This allows the Durable Object to be evicted from memory during inactivity
        self.state.accept_web_socket(&server);

        console_log!(
            "New WebSocket connection accepted. Total active connections: {}",
            self.state.get_websockets().len()
        );

        // Return the client WebSocket to the caller
        Response::from_websocket(client)
    }

    // Handle incoming WebSocket messages
    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        // Extract text from message
        let text = match message {
            WebSocketIncomingMessage::String(s) => s,
            WebSocketIncomingMessage::Binary(_) => {
                console_log!("Received binary message, ignoring");
                return Ok(());
            }
        };

        console_log!("Received message: {}", text);

        // Parse and handle the message
        match serde_json::from_str::<WsMessage>(&text) {
            Ok(ws_msg) => {
                match ws_msg {
                    WsMessage::Subscribe { market_id } => {
                        console_log!("Client subscribed to market: {}", market_id);

                        // Send confirmation back to this client
                        let response = serde_json::to_string(&WsMessage::Subscribed {
                            market_id: market_id.clone(),
                        })
                        .unwrap();

                        let _ = ws.send_with_str(&response);
                    }
                    WsMessage::Ping => {
                        // Respond with pong
                        let response = serde_json::to_string(&WsMessage::Pong).unwrap();
                        let _ = ws.send_with_str(&response);
                    }
                    _ => {
                        // Broadcast other messages to all connected clients
                        self.broadcast(&text)?;
                    }
                }
            }
            Err(e) => {
                console_log!("Failed to parse message: {}", e);
            }
        }

        Ok(())
    }

    // Handle WebSocket close events
    async fn websocket_close(
        &self,
        ws: WebSocket,
        code: usize,
        reason: String,
        was_clean: bool,
    ) -> Result<()> {
        console_log!(
            "WebSocket closed: code={}, reason={}, clean={}, remaining connections: {}",
            code,
            reason,
            was_clean,
            self.state.get_websockets().len()
        );

        // The runtime automatically removes the websocket from the hibernation set
        // No manual cleanup needed
        let _ = ws; // Suppress unused variable warning

        Ok(())
    }

    // Handle WebSocket errors
    async fn websocket_error(&self, ws: WebSocket, error: Error) -> Result<()> {
        console_log!("WebSocket error: {:?}", error);

        // Try to close the websocket gracefully
        let _ = ws.close(Some(1011), Some("Internal error".to_string()));

        Ok(())
    }
}

impl CazinoRoom {
    /// Broadcast a message to all connected sessions
    pub fn broadcast(&self, message: &str) -> Result<()> {
        // Get all connected websockets from the hibernation state
        let websockets = self.state.get_websockets();

        console_log!("Broadcasting to {} sessions", websockets.len());

        // Send to all connected clients
        // The runtime handles disconnected sockets automatically
        for ws in websockets {
            if let Err(e) = ws.send_with_str(message) {
                console_log!("Failed to send to websocket: {:?}", e);
                // Continue sending to other clients even if one fails
            }
        }

        Ok(())
    }
}
