use crate::game_manager::GameManager;
use crate::models::{StateManager, WebSocketSender};
use crate::handlers::{HandlerContext, RoomHandler, GameHandler, DrawHandler, RematchHandler, LobbyHandler, PlayerHandler};
use std::sync::Arc;
use tokio::sync::RwLock;

// Import and re-export message types (used internally and by main.rs)
pub use crate::messages::{ClientMessage, ServerMessage};

pub struct WebSocketHandler {
    ctx: HandlerContext,
}

impl WebSocketHandler {
    pub fn new(
        player_id: String,
        game_manager: Arc<RwLock<GameManager>>,
        state_manager: Arc<RwLock<StateManager>>,
        sender: WebSocketSender,
    ) -> Self {
        Self {
            ctx: HandlerContext::new(player_id, game_manager, state_manager, sender),
        }
    }

    pub async fn handle_message(&mut self, msg: ClientMessage) -> Result<(), axum::Error> {
        match msg {
            ClientMessage::CreateRoom { password, allow_spectators, time_control } => {
                RoomHandler::new(&self.ctx).create_room(password, allow_spectators, time_control).await
            }
            ClientMessage::JoinRoom { room_id, password } => {
                RoomHandler::new(&self.ctx).join_room(room_id, password).await
            }
            ClientMessage::SpectateRoom { room_id, password } => {
                RoomHandler::new(&self.ctx).spectate_room(room_id, password).await
            }
            ClientMessage::GetGamesList => {
                LobbyHandler::new(&self.ctx).get_games_list().await
            }
            ClientMessage::JoinGame => {
                LobbyHandler::new(&self.ctx).join_game().await
            }
            ClientMessage::GetLobbyStatus => {
                LobbyHandler::new(&self.ctx).get_lobby_status().await
            }
            ClientMessage::MakeMove { move_uci } => {
                GameHandler::new(&self.ctx).make_move(move_uci).await
            }
            ClientMessage::GetLegalMoves { square } => {
                GameHandler::new(&self.ctx).get_legal_moves(square).await
            }
            ClientMessage::PlayerReady => {
                GameHandler::new(&self.ctx).player_ready().await
            }
            ClientMessage::GetGameState => {
                GameHandler::new(&self.ctx).get_game_state().await
            }
            ClientMessage::OfferDraw => {
                DrawHandler::new(&self.ctx).offer_draw().await
            }
            ClientMessage::AcceptDraw => {
                DrawHandler::new(&self.ctx).accept_draw().await
            }
            ClientMessage::Resign => {
                DrawHandler::new(&self.ctx).resign().await
            }
            ClientMessage::ClaimThreefoldRepetition => {
                DrawHandler::new(&self.ctx).claim_threefold().await
            }
            ClientMessage::ClaimFiftyMoves => {
                DrawHandler::new(&self.ctx).claim_fifty_moves().await
            }
            ClientMessage::RequestRematch => {
                RematchHandler::new(&self.ctx).request_rematch().await
            }
            ClientMessage::AcceptRematch => {
                RematchHandler::new(&self.ctx).accept_rematch().await
            }
            ClientMessage::RejectRematch => {
                RematchHandler::new(&self.ctx).reject_rematch().await
            }
            ClientMessage::SetPlayerName { name } => {
                PlayerHandler::new(&self.ctx).set_player_name(name).await
            }
            ClientMessage::LeaveGame => {
                PlayerHandler::new(&self.ctx).leave_game().await
            }
        }
    }
}
