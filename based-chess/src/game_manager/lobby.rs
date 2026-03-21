use super::GameManager;
use super::types::{LobbyStatus, QueueResult};
use crate::chess_game::ChessGame;
use rand::seq::SliceRandom;
use uuid::Uuid;

impl GameManager {
    pub fn join_lobby(&mut self, player_id: String) -> LobbyStatus {
        if let Some(_) = self.player_to_game.get(&player_id) {
            self.forfeit_game_for_player(&player_id);
        }

        // Remove from spectator tracking if they were spectating
        if let Some(game_id) = self.spectator_to_game.remove(&player_id) {
            if let Some(spectators) = self.spectators.get_mut(&game_id) {
                spectators.remove(&player_id);
                if spectators.is_empty() {
                    self.spectators.remove(&game_id);
                }
            }
        }

        self.lobby_queue.retain(|id| id != &player_id);
        
        // Clean up any empty games (games with zero players)
        self.cleanup_empty_games();

        LobbyStatus::InLobby {
            players_in_queue: self.lobby_queue.len(),
            active_games_count: self.get_active_games_count(),
        }
    }
    
    pub fn join_game_queue(&mut self, player_id: String) -> QueueResult {
        if let Some(_) = self.player_to_game.get(&player_id) {
            self.forfeit_game_for_player(&player_id);
        }
        
        if !self.lobby_queue.contains(&player_id) {
            self.lobby_queue.push(player_id.clone());
        }
        
        if self.lobby_queue.len() >= 2 {
            let mut rng = rand::thread_rng();
            let mut players = self.lobby_queue.clone();
            players.shuffle(&mut rng);
            
            let white_player = players[0].clone();
            let black_player = players[1].clone();
            
            self.lobby_queue.retain(|id| id != &white_player && id != &black_player);
            
            let game_id = Uuid::new_v4().to_string();
            let room_id = Uuid::new_v4().to_string();
            
            // Default to Classical time control for queue matches
            let mut game = ChessGame::new(game_id.clone(), room_id.clone(), None, true, crate::chess_game::TimeControl::Classical);
            game.assign_players(white_player.clone(), Some(black_player.clone()));
            
            self.games.insert(game_id.clone(), game);
            self.player_to_game.insert(white_player.clone(), game_id.clone());
            self.player_to_game.insert(black_player.clone(), game_id.clone());
            
            if player_id == white_player {
                QueueResult::Matched {
                    game_id,
                    room_id,
                    white_player,
                    black_player,
                    color: "white".to_string(),
                    game_started: false,
                }
            } else {
                QueueResult::Matched {
                    game_id,
                    room_id,
                    white_player,
                    black_player,
                    color: "black".to_string(),
                    game_started: false,
                }
            }
        } else {
            let position = self.lobby_queue.iter().position(|id| id == &player_id).unwrap_or(0) + 1;
            QueueResult::InQueue {
                players_in_queue: self.lobby_queue.len(),
                position,
            }
        }
    }
    
    pub fn get_lobby_status(&mut self, player_id: &str) -> LobbyStatus {
        let active_games_count = self.get_active_games_count();
        
        if let Some(game_id) = self.player_to_game.get(player_id).cloned() {
            if let Some(game) = self.games.get(&game_id) {
                // Defensive check: verify player is actually in the game
                let player_in_game = if game.colors_assigned {
                    Some(player_id) == game.white_player.as_deref() || Some(player_id) == game.black_player.as_deref()
                } else {
                    Some(player_id) == game.player1.as_deref() || Some(player_id) == game.player2.as_deref()
                };
                
                if !player_in_game {
                    // Orphaned mapping - player is in player_to_game but not actually in the game
                    // Clean up the orphaned mapping
                    self.player_to_game.remove(player_id);
                } else if !game.is_game_over() {
                    let player_color = game.get_player_color(player_id);
                    let waiting_for_opponent = if game.colors_assigned {
                        if Some(player_id) == game.white_player.as_deref() {
                            game.black_player.is_none()
                        } else {
                            game.white_player.is_none()
                        }
                    } else {
                        if Some(player_id) == game.player1.as_deref() {
                            game.player2.is_none()
                        } else {
                            game.player1.is_none()
                        }
                    };
                    
                    return LobbyStatus::InGame {
                        game_id: game_id.clone(),
                        room_id: game.room_id.clone(),
                        color: player_color,
                        waiting_for_opponent,
                        game_started: game.game_started,
                        active_games_count,
                    };
                }
            } else {
                // Game doesn't exist but player is still mapped to it - orphaned mapping
                self.player_to_game.remove(player_id);
            }
        }
        
        if let Some(position) = self.lobby_queue.iter().position(|id| id == player_id) {
            LobbyStatus::InQueue {
                players_in_queue: self.lobby_queue.len(),
                position: position + 1,
                active_games_count,
            }
        } else {
            LobbyStatus::InLobby {
                players_in_queue: self.lobby_queue.len(),
                active_games_count,
            }
        }
    }

    pub fn get_all_games(&self) -> Vec<crate::utils::game_helpers::GameInfo> {
        let mut games_list = Vec::new();
        
        for game in self.games.values() {
            // Verify players are actually in player_to_game mapping
            let player1_valid = game.player1.as_ref()
                .map(|p| self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let player2_valid = game.player2.as_ref()
                .map(|p| self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let white_player_valid = game.white_player.as_ref()
                .map(|p| self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let black_player_valid = game.black_player.as_ref()
                .map(|p| self.player_to_game.contains_key(p))
                .unwrap_or(false);
            
            // Skip games where players are listed but not actually in player_to_game
            if (game.player1.is_some() && !player1_valid) ||
               (game.player2.is_some() && !player2_valid) ||
               (game.white_player.is_some() && !white_player_valid) ||
               (game.black_player.is_some() && !black_player_valid) {
                continue;
            }
            
            // Skip games with no valid players
            if game.player1.is_none() && game.player2.is_none() && 
               game.white_player.is_none() && game.black_player.is_none() {
                continue;
            }
            
            // Skip games that are over AND have no players (but include games waiting for rematch)
            // Games waiting for rematch are over but still have players, so they should be visible
            if game.is_game_over() {
                // Check if game has any valid players - if so, it's waiting for rematch and should be visible
                let has_valid_players = player1_valid || player2_valid || white_player_valid || black_player_valid;
                // Also check if there are rematch requests pending
                let has_rematch_pending = game.rematch_requested_by.is_some() || game.rematch_accepted_by.is_some();
                
                // Only skip if game is over AND has no valid players AND no rematch pending
                if !has_valid_players && !has_rematch_pending {
                    continue;
                }
            }
            
            let waiting_for_player = if game.colors_assigned {
                (game.black_player.is_none() && game.white_player.is_some()) ||
                (game.white_player.is_none() && game.black_player.is_some())
            } else {
                (game.player2.is_none() && game.player1.is_some()) ||
                (game.player1.is_none() && game.player2.is_some())
            };
            
            let spectator_count = self.spectators.get(&game.game_id).map(|s| s.len()).unwrap_or(0);
            
            let time_control_str = match game.time_control {
                crate::chess_game::TimeControl::Bullet => Some("bullet".to_string()),
                crate::chess_game::TimeControl::Blitz => Some("blitz".to_string()),
                crate::chess_game::TimeControl::Rapid => Some("rapid".to_string()),
                crate::chess_game::TimeControl::Classical => Some("classical".to_string()),
            };
            
            games_list.push(crate::utils::game_helpers::GameInfo {
                game_id: game.game_id.clone(),
                room_id: game.room_id.clone(),
                white_player: game.white_player.clone(),
                black_player: game.black_player.clone(),
                player1: game.player1.clone(),
                player2: game.player2.clone(),
                game_started: game.game_started,
                waiting_for_player,
                has_password: game.has_password(),
                allow_spectators: game.allow_spectators,
                white_player_name: None,
                black_player_name: None,
                spectator_count,
                time_control: time_control_str,
            });
        }
        
        games_list
    }
}

