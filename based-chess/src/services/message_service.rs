use crate::messages::ServerMessage;
use crate::models::{StateManager, WebSocketSender};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service for sending messages to players and broadcasting to multiple players
pub struct MessageService {
    state_manager: Arc<RwLock<StateManager>>,
}

impl MessageService {
    pub fn new(state_manager: Arc<RwLock<StateManager>>) -> Self {
        Self { state_manager }
    }

    /// Send a message to a specific player
    pub async fn send_to_player(
        &self,
        player_id: &str,
        message: &ServerMessage,
    ) -> Result<(), axum::Error> {
        if let Some(sender) = self.state_manager.read().await.get_connection(player_id).await {
            crate::messages::server::send_json(&sender, message)
        } else {
            Err(axum::Error::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Player {} not found", player_id),
            )))
        }
    }

    /// Broadcast a message to multiple players
    pub async fn broadcast_to_players(
        &self,
        player_ids: &[String],
        message: &ServerMessage,
    ) {
        let sm = self.state_manager.read().await;
        for player_id in player_ids {
            if let Some(sender) = sm.get_connection(player_id).await {
                let _ = crate::messages::server::send_json(&sender, message);
            }
        }
    }

    /// Broadcast a message to all connected players
    pub async fn broadcast_to_all(&self, message: &ServerMessage) {
        let exclude = HashSet::new();
        let connections = self.state_manager.read().await.get_connections_excluding(&exclude).await;
        for (_, sender) in connections {
            let _ = crate::messages::server::send_json(&sender, message);
        }
    }

    /// Send a message using a direct sender (for cases where we already have the sender)
    pub fn send_direct(sender: &WebSocketSender, message: &ServerMessage) -> Result<(), axum::Error> {
        crate::messages::server::send_json(sender, message)
    }
}

