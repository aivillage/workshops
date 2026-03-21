use crate::game_manager::{LobbyStatus, QueueResult};
use crate::models::WebSocketSender;
use crate::utils::game_helpers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "connected")]
    Connected {
        player_id: String,
        player_name: String,
    },
    #[serde(rename = "lobby_status")]
    LobbyStatus {
        #[serde(flatten)]
        status: LobbyStatus,
    },
    #[serde(rename = "games_list")]
    GamesList {
        games: Vec<game_helpers::GameInfo>,
    },
    #[serde(rename = "room_created")]
    RoomCreated {
        game_id: String,
        room_id: String,
        white_player: Option<String>,
        black_player: Option<String>,
        color: Option<String>,
        game_started: bool,
    },
    #[serde(rename = "room_joined")]
    RoomJoined {
        game_id: String,
        room_id: String,
        white_player: Option<String>,
        black_player: Option<String>,
        color: Option<String>,
        game_started: bool,
    },
    #[serde(rename = "matched")]
    Matched {
        game_id: String,
        room_id: String,
        white_player: String,
        black_player: String,
        color: String,
        game_started: bool,
    },
    #[serde(rename = "queue_status")]
    QueueStatus {
        #[serde(flatten)]
        status: QueueResult,
    },
    #[serde(rename = "game_state")]
    GameState {
        game_id: String,
        room_id: String,
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        board: std::collections::HashMap<String, crate::chess_game::PieceInfo>,
        turn: Option<String>,
        your_color: Option<String>,
        is_game_over: bool,
        is_check: bool,
        is_checkmate: bool,
        is_stalemate: bool,
        result: Option<String>,
        result_reason: Option<String>,
        result_message: Option<String>,
        fen: String,
        waiting_for_opponent: bool,
        game_started: bool,
        white_ready: bool,
        black_ready: bool,
        draw_offered_by: Option<String>,
        resigned_by: Option<String>,
        accept_draw_called: bool,
        can_claim_threefold_repetition: bool,
        can_claim_fifty_moves: bool,
        can_claim_draw: bool,
        claimed_draw_by: Option<String>,
        claimed_draw_reason: Option<String>,
        can_rematch: bool,
        rematch_requested_by: Option<String>,
        rematch_accepted_by: Option<String>,
        opponent_name: Option<String>,
        white_player_name: Option<String>,
        black_player_name: Option<String>,
        white_player: Option<String>,
        black_player: Option<String>,
        move_history: Vec<String>,
        move_san_history: Vec<String>,
        move_fens: Vec<String>,
        spectator_count: usize,
        is_spectator: bool,
        time_control: Option<String>,
        white_time_remaining: Option<u64>,
        black_time_remaining: Option<u64>,
    },
    #[serde(rename = "room_spectated")]
    RoomSpectated {
        game_id: String,
        room_id: String,
        white_player: Option<String>,
        black_player: Option<String>,
        color: Option<String>,
        game_started: bool,
    },
    #[serde(rename = "game_started")]
    GameStarted {
        your_color: String,
        white_player: Option<String>,
        black_player: Option<String>,
        white_player_name: Option<String>,
        black_player_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "opponent_joined")]
    OpponentJoined {
        game_id: String,
        room_id: String,
        opponent_name: Option<String>,
        opponent_color: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "opponent_left")]
    OpponentLeft {
        opponent_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "opponent_disconnected")]
    OpponentDisconnected {
        opponent_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "move_made")]
    MoveMade {
        move_uci: String,
        player_name: Option<String>,
        player_color: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "legal_moves")]
    LegalMoves {
        square: String,
        valid_squares: Vec<String>,
        legal_moves: Vec<String>,
    },
    #[serde(rename = "draw_offered")]
    DrawOffered {
        offered_by: String,
        offered_by_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "draw_accepted")]
    DrawAccepted {
        accepted_by: Option<String>,
        accepted_by_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "resignation")]
    Resignation {
        resigned_by: String,
        resigned_by_name: Option<String>,
        winner_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "draw_claimed")]
    DrawClaimed {
        claimed_by: String,
        claimed_by_name: Option<String>,
        reason: String,
        message: Option<String>,
    },
    #[serde(rename = "rematch_requested")]
    RematchRequested {
        requested_by: Option<String>,
        requested_by_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "rematch_accepted")]
    RematchAccepted {
        accepted_by: Option<String>,
        accepted_by_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "rematch_started")]
    RematchStarted {
        your_color: String,
        white_player_name: Option<String>,
        black_player_name: Option<String>,
        message: Option<String>,
    },
    #[serde(rename = "rematch_rejected")]
    RematchRejected {
        message: String,
    },
    #[serde(rename = "rematch_timeout")]
    RematchTimeout {
        message: String,
    },
    #[serde(rename = "kicked_to_lobby")]
    KickedToLobby {
        message: String,
    },
    #[serde(rename = "left_game")]
    LeftGame,
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// Send a JSON message to a WebSocket connection
pub fn send_json(ws: &WebSocketSender, msg: &ServerMessage) -> Result<(), axum::Error> {
    let json = serde_json::to_string(msg)
        .map_err(|e| axum::Error::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Serialization error: {}", e)
        )))?;
    ws.send(axum::extract::ws::Message::Text(json))
        .map_err(|_| axum::Error::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to send message"
        )))
}

