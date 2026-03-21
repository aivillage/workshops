use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeControl {
    Bullet,
    Blitz,
    Rapid,
    Classical,
}

impl TimeControl {
    pub fn initial_time_seconds(&self) -> u64 {
        match self {
            TimeControl::Bullet => 120,      // 2 minutes (Bullet 2+1)
            TimeControl::Blitz => 180,       // 3 minutes (Blitz 3+2)
            TimeControl::Rapid => 900,       // 15 minutes (Rapid 15+10)
            TimeControl::Classical => 3600,  // 60 minutes (Classical 60+30)
        }
    }

    /// Increment in seconds added after each move (FIDE-style base+increment)
    pub fn increment_seconds(&self) -> u64 {
        match self {
            TimeControl::Bullet => 1,   // 2+1
            TimeControl::Blitz => 2,    // 3+2
            TimeControl::Rapid => 10,   // 15+10
            TimeControl::Classical => 30, // 60+30
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveHistoryEntry {
    pub uci: String,
    pub san: String,
    pub move_number: usize,
    pub is_white: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieceInfo {
    #[serde(rename = "type")]
    pub piece_type: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStatus {
    pub is_game_over: bool,
    pub is_check: bool,
    pub is_checkmate: bool,
    pub is_stalemate: bool,
    pub is_insufficient_material: bool,
    pub turn: String,
    pub result: Option<String>,
    pub result_reason: Option<String>,
}

