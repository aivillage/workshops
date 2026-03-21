use crate::game_manager::GameManager;
use crate::models::{StateManager, WebSocketSender};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared context for all handlers
pub struct HandlerContext {
    pub player_id: String,
    pub game_manager: Arc<RwLock<GameManager>>,
    pub state_manager: Arc<RwLock<StateManager>>,
    pub sender: WebSocketSender,
}

impl HandlerContext {
    pub fn new(
        player_id: String,
        game_manager: Arc<RwLock<GameManager>>,
        state_manager: Arc<RwLock<StateManager>>,
        sender: WebSocketSender,
    ) -> Self {
        Self {
            player_id,
            game_manager,
            state_manager,
            sender,
        }
    }
}

