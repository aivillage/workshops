use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use axum::extract::ws::Message;

// Type alias for WebSocket sender
pub type WebSocketSender = mpsc::UnboundedSender<Message>;

pub struct StateManager {
    player_names: Arc<RwLock<HashMap<String, String>>>,
    connections: Arc<RwLock<HashMap<String, WebSocketSender>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            player_names: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_player_name(&self, player_id: &str) -> Option<String> {
        self.player_names.read().await.get(player_id).cloned()
    }

    pub async fn set_player_name(&self, player_id: String, name: String) {
        self.player_names.write().await.insert(player_id, name);
    }

    pub async fn remove_player_name(&self, player_id: &str) {
        self.player_names.write().await.remove(player_id);
    }

    /// Fetch names only for the given player IDs (sparse lookup for scalability).
    pub async fn get_player_names_for_ids(
        &self,
        ids: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> HashMap<String, String> {
        let names = self.player_names.read().await;
        ids.into_iter()
            .filter_map(|id| {
                names
                    .get(id.as_ref())
                    .map(|n| (id.as_ref().to_string(), n.clone()))
            })
            .collect()
    }

    pub async fn set_connection(&self, player_id: String, sender: WebSocketSender) {
        self.connections.write().await.insert(player_id, sender);
    }

    pub async fn get_connection(&self, player_id: &str) -> Option<WebSocketSender> {
        self.connections.read().await.get(player_id).cloned()
    }

    pub async fn remove_connection(&self, player_id: &str) {
        self.connections.write().await.remove(player_id);
    }

    /// Get connections excluding the given IDs. Returns Vec to avoid HashMap clone.
    pub async fn get_connections_excluding(
        &self,
        exclude: &HashSet<String>,
    ) -> Vec<(String, WebSocketSender)> {
        let conns = self.connections.read().await;
        conns
            .iter()
            .filter(|(id, _)| !exclude.contains(id.as_str()))
            .map(|(id, s)| (id.clone(), s.clone()))
            .collect()
    }

    pub async fn has_connection(&self, player_id: &str) -> bool {
        self.connections.read().await.contains_key(player_id)
    }
}

