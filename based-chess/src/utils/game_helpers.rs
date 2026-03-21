use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Collect unique player IDs from games for sparse name lookups.
pub fn collect_player_ids_from_games(games: &[GameInfo]) -> Vec<String> {
    let mut ids = HashSet::new();
    for game in games {
        ids.extend(
            game.white_player
                .iter()
                .chain(game.black_player.iter())
                .chain(game.player1.iter())
                .chain(game.player2.iter())
                .cloned(),
        );
    }
    ids.into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub game_id: String,
    pub room_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_player: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub black_player: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player2: Option<String>,
    pub game_started: bool,
    pub waiting_for_player: bool,
    pub has_password: bool,
    pub allow_spectators: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_player_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub black_player_name: Option<String>,
    pub spectator_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_control: Option<String>,
}

pub fn add_names_to_games(
    games_list: Vec<GameInfo>,
    player_names: &HashMap<String, String>,
) -> Vec<GameInfo> {
    games_list
        .into_iter()
        .map(|mut game| {
            // If colors are assigned, use white_player/black_player
            if game.white_player.is_some() && game.black_player.is_some() {
                game.white_player_name = game.white_player.as_ref().and_then(|id| {
                    player_names.get(id).cloned().or_else(|| {
                        Some(format!("Anonymous {}", &id.chars().take(8).collect::<String>()))
                    })
                });
                game.black_player_name = game.black_player.as_ref().and_then(|id| {
                    player_names.get(id).cloned().or_else(|| {
                        Some(format!("Anonymous {}", &id.chars().take(8).collect::<String>()))
                    })
                });
            } else {
                // Colors not assigned yet, use player1/player2
                game.white_player_name = game.player1.as_ref().and_then(|id| {
                    player_names.get(id).cloned().or_else(|| {
                        Some(format!("Anonymous {}", &id.chars().take(8).collect::<String>()))
                    })
                }).or(Some("Waiting...".to_string()));
                
                game.black_player_name = game.player2.as_ref().and_then(|id| {
                    player_names.get(id).cloned().or_else(|| {
                        Some(format!("Anonymous {}", &id.chars().take(8).collect::<String>()))
                    })
                }).or(Some("Waiting...".to_string()));
            }
            game
        })
        .collect()
}

