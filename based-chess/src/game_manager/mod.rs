use crate::chess_game::ChessGame;
use std::collections::{HashMap, HashSet};

pub mod types;
pub mod lobby;
pub mod rooms;
pub mod games;
pub mod spectators;

pub use types::*;

#[derive(Debug, Clone)]
pub struct GameManager {
    pub(super) games: HashMap<String, ChessGame>,
    pub(super) player_to_game: HashMap<String, String>,
    pub(super) lobby_queue: Vec<String>,
    pub(super) spectators: HashMap<String, HashSet<String>>, // game_id -> set of spectator_ids
    pub(super) spectator_to_game: HashMap<String, String>, // spectator_id -> game_id
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            player_to_game: HashMap::new(),
            lobby_queue: Vec::new(),
            spectators: HashMap::new(),
            spectator_to_game: HashMap::new(),
        }
    }
}

