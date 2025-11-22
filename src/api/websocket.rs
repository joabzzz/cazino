/// WebSocket handler for real-time updates
use crate::api::models::WsMessage;
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::error;

/// Broadcast channel for market updates
pub type BroadcastTx = broadcast::Sender<WsMessage>;
pub type BroadcastRx = broadcast::Receiver<WsMessage>;

/// Create a new broadcast channel for WebSocket messages
pub fn create_broadcast_channel() -> (BroadcastTx, BroadcastRx) {
    broadcast::channel(1000)
}

/// Handle a WebSocket connection
pub async fn handle_socket(socket: WebSocket, broadcast_tx: Arc<BroadcastTx>) {
    tracing::info!("🔌 New WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = broadcast_tx.subscribe();

    // Spawn task to receive broadcast messages and send to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Spawn task to receive messages from client
    let broadcast_tx_clone = Arc::clone(&broadcast_tx);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_client_message(&text, &broadcast_tx_clone).await;
                }
                Message::Ping(_) => {
                    // Echo pong is handled automatically by Axum
                    tracing::debug!("Received WebSocket ping");
                }
                Message::Close(_) => {
                    tracing::info!("Client initiated WebSocket close");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    tracing::info!("🔌 WebSocket connection closed");
}

async fn handle_client_message(text: &str, _broadcast_tx: &BroadcastTx) {
    match serde_json::from_str::<WsMessage>(text) {
        Ok(WsMessage::Ping) => {
            tracing::debug!("Received ping from client");
            // Could send pong back if needed
        }
        Ok(WsMessage::Subscribe { market_id }) => {
            tracing::info!("📺 Client subscribed to market: {}", market_id);
            // In a production system, you'd track subscriptions per market
            // For MVP, we broadcast all updates to all connected clients
        }
        Ok(msg) => {
            tracing::debug!("Received message from client: {:?}", msg);
        }
        Err(e) => {
            tracing::warn!("Failed to parse client message: {} | Text: {}", e, text);
        }
    }
}

/// Broadcast a message to all connected WebSocket clients
pub fn broadcast(tx: &BroadcastTx, message: WsMessage) {
    match tx.send(message) {
        Ok(receiver_count) => {
            if receiver_count > 0 {
                tracing::debug!("📡 Broadcasted to {} WebSocket client(s)", receiver_count);
            }
            // If receiver_count is 0, no clients connected - this is normal during HTTP-only testing
            // Don't log anything to avoid spam
        }
        Err(_) => {
            // Channel closed - this shouldn't happen in normal operation
            // Only happens if broadcast channel is dropped, which is a real error
            tracing::error!("Broadcast channel closed - this shouldn't happen!");
        }
    }
}
