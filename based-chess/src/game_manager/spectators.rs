use super::GameManager;
use std::collections::HashSet;

impl GameManager {
    pub fn get_spectator_count(&self, game_id: &str) -> usize {
        self.spectators.get(game_id).map(|s| s.len()).unwrap_or(0)
    }
    
    pub fn get_spectators(&self, game_id: &str) -> Vec<String> {
        self.spectators.get(game_id).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn get_spectated_game_for_player(&self, player_id: &str) -> Option<String> {
        self.spectator_to_game.get(player_id).cloned()
    }

    pub fn remove_spectator(&mut self, player_id: &str) {
        if let Some(game_id) = self.spectator_to_game.remove(player_id) {
            if let Some(spectators) = self.spectators.get_mut(&game_id) {
                spectators.remove(player_id);
                if spectators.is_empty() {
                    self.spectators.remove(&game_id);
                }
            }
        }
    }
}

