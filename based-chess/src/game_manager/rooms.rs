use super::GameManager;
use super::types::RoomResult;
use crate::chess_game::{ChessGame, TimeControl};
use uuid::Uuid;

impl GameManager {
    pub fn create_room(
        &mut self,
        player_id: String,
        password: Option<String>,
        allow_spectators: bool,
        time_control: TimeControl,
    ) -> RoomResult {
        if let Some(_) = self.player_to_game.get(&player_id) {
            self.forfeit_game_for_player(&player_id);
        }

        self.lobby_queue.retain(|id| id != &player_id);
        
        // Clean up any empty games before creating new one
        self.cleanup_empty_games();

        if let Some(ref pwd) = password {
            if pwd.len() > 20 {
                return RoomResult::Error {
                    error: "Password must be 20 characters or less".to_string(),
                };
            }
        }

        let game_id = Uuid::new_v4().to_string();
        let room_id = Uuid::new_v4().to_string();

        let mut game = ChessGame::new(game_id.clone(), room_id.clone(), password, allow_spectators, time_control);
        if game.add_player(player_id.clone()).is_err() {
            return RoomResult::Error {
                error: "Failed to add player".to_string(),
            };
        }

        self.games.insert(game_id.clone(), game);
        self.player_to_game.insert(player_id, game_id.clone());

        RoomResult::Success {
            status: "room_created".to_string(),
            game_id,
            room_id,
            white_player: None,
            black_player: None,
            color: None,
            game_started: false,
        }
    }

    pub fn join_room(
        &mut self,
        player_id: String,
        room_id: String,
        password: Option<String>,
    ) -> RoomResult {
        if let Some(_) = self.player_to_game.get(&player_id) {
            self.forfeit_game_for_player(&player_id);
        }

        self.lobby_queue.retain(|id| id != &player_id);
        
        // Clean up any empty games before joining
        self.cleanup_empty_games();

        let game = match self.get_game_by_room_id(&room_id) {
            Some(g) => g,
            None => {
                return RoomResult::Error {
                    error: "Room not found".to_string(),
                };
            }
        };

        if game.is_game_over() {
            return RoomResult::Error {
                error: "Game has ended".to_string(),
            };
        }

        if game.player1.is_some() && game.player2.is_some() {
            return RoomResult::Error {
                error: "Room is full".to_string(),
            };
        }

        if Some(&player_id) == game.player1.as_ref() || Some(&player_id) == game.player2.as_ref() {
            return RoomResult::Error {
                error: "You are already in this room".to_string(),
            };
        }

        if game.has_password() {
            if password.is_none() || password.as_ref() != game.password.as_ref() {
                return RoomResult::Error {
                    error: "Incorrect password".to_string(),
                };
            }
        }

        let game_id = game.game_id.clone();
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.add_player(player_id.clone()).is_err() {
                return RoomResult::Error {
                    error: "Room is full".to_string(),
                };
            }
            self.player_to_game.insert(player_id, game_id.clone());
        }

        RoomResult::Success {
            status: "room_joined".to_string(),
            game_id,
            room_id,
            white_player: None,
            black_player: None,
            color: None,
            game_started: false,
        }
    }

    pub fn spectate_room(
        &mut self,
        spectator_id: String,
        room_id: String,
        password: Option<String>,
    ) -> RoomResult {
        // Remove from any previous game/spectating
        if let Some(_) = self.player_to_game.get(&spectator_id) {
            self.forfeit_game_for_player(&spectator_id);
        }
        
        // Remove from previous spectating if any
        if let Some(old_game_id) = self.spectator_to_game.remove(&spectator_id) {
            if let Some(spectators) = self.spectators.get_mut(&old_game_id) {
                spectators.remove(&spectator_id);
                if spectators.is_empty() {
                    self.spectators.remove(&old_game_id);
                }
            }
        }

        self.lobby_queue.retain(|id| id != &spectator_id);
        
        // Clean up any empty games before spectating
        self.cleanup_empty_games();

        let game = match self.get_game_by_room_id(&room_id) {
            Some(g) => g,
            None => {
                return RoomResult::Error {
                    error: "Room not found".to_string(),
                };
            }
        };

        // Check if spectators are allowed
        if !game.allow_spectators {
            return RoomResult::Error {
                error: "Spectators are not allowed in this room".to_string(),
            };
        }

        // Allow spectating games that are over but still have players (waiting for rematch)
        // Only block spectating if game is over AND has no players
        if game.is_game_over() {
            let has_players = game.player1.is_some() || game.player2.is_some() || 
                             game.white_player.is_some() || game.black_player.is_some();
            let has_rematch_pending = game.rematch_requested_by.is_some() || game.rematch_accepted_by.is_some();
            
            // Only block if game is over AND has no players AND no rematch pending
            if !has_players && !has_rematch_pending {
                return RoomResult::Error {
                    error: "Game has ended".to_string(),
                };
            }
        }

        // Check if already a player in this game
        if Some(&spectator_id) == game.player1.as_ref() || 
           Some(&spectator_id) == game.player2.as_ref() ||
           Some(&spectator_id) == game.white_player.as_ref() ||
           Some(&spectator_id) == game.black_player.as_ref() {
            return RoomResult::Error {
                error: "You are already a player in this room".to_string(),
            };
        }

        if game.has_password() {
            if password.is_none() || password.as_ref() != game.password.as_ref() {
                return RoomResult::Error {
                    error: "Incorrect password".to_string(),
                };
            }
        }

        let game_id = game.game_id.clone();
        
        // Add spectator
        self.spectators.entry(game_id.clone()).or_insert_with(std::collections::HashSet::new).insert(spectator_id.clone());
        self.spectator_to_game.insert(spectator_id, game_id.clone());

        RoomResult::Success {
            status: "spectating".to_string(),
            game_id,
            room_id,
            white_player: None,
            black_player: None,
            color: None,
            game_started: false,
        }
    }
}

