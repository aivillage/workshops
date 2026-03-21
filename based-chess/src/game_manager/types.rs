use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum LobbyStatus {
    #[serde(rename = "in_lobby")]
    InLobby {
        players_in_queue: usize,
        active_games_count: usize,
    },
    #[serde(rename = "in_queue")]
    InQueue {
        players_in_queue: usize,
        position: usize,
        active_games_count: usize,
    },
    #[serde(rename = "in_game")]
    InGame {
        game_id: String,
        room_id: String,
        color: Option<String>,
        waiting_for_opponent: bool,
        game_started: bool,
        active_games_count: usize,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RoomResult {
    Success {
        status: String,
        game_id: String,
        room_id: String,
        white_player: Option<String>,
        black_player: Option<String>,
        color: Option<String>,
        game_started: bool,
    },
    Error {
        error: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum QueueResult {
    #[serde(rename = "matched")]
    Matched {
        game_id: String,
        room_id: String,
        white_player: String,
        black_player: String,
        color: String,
        game_started: bool,
    },
    #[serde(rename = "in_queue")]
    InQueue {
        players_in_queue: usize,
        position: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ReadyResult {
    pub game_id: String,
    pub game_started: bool,
    pub white_ready: bool,
    pub black_ready: bool,
    pub white_player: Option<String>,
    pub black_player: Option<String>,
    pub your_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DrawResult {
    pub game_id: String,
    pub draw_offered_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DrawAcceptResult {
    pub game_id: String,
    pub draw_accepted: bool,
}

#[derive(Debug, Clone)]
pub struct ResignResult {
    pub game_id: String,
    pub resigned_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClaimDrawResult {
    pub game_id: String,
    pub claimed_draw_by: Option<String>,
    pub claimed_draw_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RematchResult {
    pub game_id: String,
    pub rematch_requested_by: Option<String>,
    pub rematch_accepted_by: Option<String>,
    pub both_accepted: bool,
}

#[derive(Debug, Clone)]
pub struct RejectRematchResult {
    pub game_id: String,
    pub is_creator: bool,
}

#[derive(Debug, Clone)]
pub struct ForfeitResult {
    pub game_id: String,
    pub room_id: String,
    pub opponent_id: Option<String>,
    pub resigned_by: Option<String>,
    pub winner_id: Option<String>,
}

