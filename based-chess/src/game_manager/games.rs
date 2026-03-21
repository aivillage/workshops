use super::GameManager;
use super::types::{ReadyResult, DrawResult, DrawAcceptResult, ResignResult, ClaimDrawResult, RematchResult, RejectRematchResult, ForfeitResult};
use crate::chess_game::ChessGame;
use std::collections::HashSet;

impl GameManager {
    /// Clean up games that have zero players or orphaned player references
    /// Returns a list of spectator IDs that were affected by game deletions
    pub fn cleanup_empty_games(&mut self) -> Vec<String> {
        let mut games_to_remove: Vec<String> = Vec::new();
        let mut affected_spectators = Vec::new();
        
        for (game_id, game) in self.games.iter() {
            // Check if game has zero players
            let has_no_players = game.player1.is_none() && game.player2.is_none();
            
            // Check if players are listed but not in player_to_game (orphaned references)
            let player1_orphaned = game.player1.as_ref()
                .map(|p| !self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let player2_orphaned = game.player2.as_ref()
                .map(|p| !self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let white_player_orphaned = game.white_player.as_ref()
                .map(|p| !self.player_to_game.contains_key(p))
                .unwrap_or(false);
            let black_player_orphaned = game.black_player.as_ref()
                .map(|p| !self.player_to_game.contains_key(p))
                .unwrap_or(false);
            
            // Game should be removed if:
            // 1. It has zero players, OR
            // 2. It has orphaned player references (players listed but not in player_to_game)
            if has_no_players || player1_orphaned || player2_orphaned || white_player_orphaned || black_player_orphaned {
                games_to_remove.push(game_id.clone());
            }
        }
        
        for game_id in games_to_remove {
            if let Some(_) = self.games.get(&game_id) {
                // Remove any remaining player mappings
                let players_to_remove: Vec<String> = self
                    .player_to_game
                    .iter()
                    .filter(|(_, gid)| *gid == &game_id)
                    .map(|(pid, _)| pid.clone())
                    .collect();
                for pid in players_to_remove {
                    self.player_to_game.remove(&pid);
                }
                
                // Remove all spectators for this game and collect them for notification
                if let Some(spectators) = self.spectators.remove(&game_id) {
                    for spectator_id in spectators.iter() {
                        self.spectator_to_game.remove(spectator_id);
                        affected_spectators.push(spectator_id.clone());
                    }
                }
                
                // Delete the game
                self.games.remove(&game_id);
            }
        }
        
        affected_spectators
    }

    pub fn get_game(&self, game_id: &str) -> Option<&ChessGame> {
        self.games.get(game_id)
    }

    pub fn get_game_mut(&mut self, game_id: &str) -> Option<&mut ChessGame> {
        self.games.get_mut(game_id)
    }

    pub fn get_game_by_room_id(&self, room_id: &str) -> Option<&ChessGame> {
        self.games.values().find(|g| g.room_id == room_id)
    }

    pub fn get_player_game(&self, player_id: &str) -> Option<&ChessGame> {
        self.player_to_game
            .get(player_id)
            .and_then(|game_id| self.games.get(game_id))
    }

    pub fn get_active_games_count(&self) -> usize {
        self.games
            .values()
            .filter(|g| g.game_started && !g.is_game_over())
            .count()
    }

    /// Player IDs currently in a game or spectating (for lobby-only broadcast optimization).
    pub fn get_busy_player_ids(&self) -> HashSet<String> {
        self.player_to_game
            .keys()
            .chain(self.spectator_to_game.keys())
            .cloned()
            .collect()
    }

    /// Check for timeouts and update timers for all active games
    /// Returns list of game IDs that need state updates sent
    pub fn check_timers(&mut self) -> Vec<String> {
        let mut games_to_update = Vec::new();
        
        for (game_id, game) in self.games.iter_mut() {
            // Only check games that are started and have timers active
            if !game.game_started || !game.timer_started || game.is_game_over() {
                continue;
            }
            
            // Calculate current time remaining
            let (white_time, black_time) = game.get_current_time_remaining();
            
            // Check for timeout on current player's turn
            let current_turn = game.get_turn();
            let timeout_occurred = if current_turn == "white" && white_time == 0 {
                // White timed out
                if let Some(white_id) = &game.white_player {
                    game.timeout_by = Some(white_id.clone());
                }
                true
            } else if current_turn == "black" && black_time == 0 {
                // Black timed out
                if let Some(black_id) = &game.black_player {
                    game.timeout_by = Some(black_id.clone());
                }
                true
            } else {
                false
            };
            
            // Always add to update list if game is active (to send time updates)
            // Or if timeout occurred (to notify of game end)
            if timeout_occurred || (white_time > 0 && black_time > 0) {
                games_to_update.push(game_id.clone());
            }
        }
        
        games_to_update
    }

    pub fn set_player_ready(&mut self, player_id: &str) -> Result<ReadyResult, String> {
        let (game_id, colors_were_assigned) = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            (game.game_id.clone(), !game.colors_assigned)
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            let game_started = game.set_player_ready(player_id);
            
            Ok(ReadyResult {
                game_id: game_id.clone(),
                game_started,
                white_ready: game.white_ready,
                black_ready: game.black_ready,
                white_player: if game_started && colors_were_assigned {
                    game.white_player.clone()
                } else {
                    None
                },
                black_player: if game_started && colors_were_assigned {
                    game.black_player.clone()
                } else {
                    None
                },
                your_color: if game_started && colors_were_assigned {
                    game.get_player_color(player_id)
                } else {
                    None
                },
            })
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn offer_draw(&mut self, player_id: &str) -> Result<DrawResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.offer_draw(player_id) {
                Ok(DrawResult {
                    game_id,
                    draw_offered_by: game.draw_offered_by.clone(),
                })
            } else {
                Err("Cannot offer draw".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn accept_draw(&mut self, player_id: &str) -> Result<DrawAcceptResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.accept_draw(player_id) {
                Ok(DrawAcceptResult {
                    game_id,
                    draw_accepted: true,
                })
            } else {
                Err("Cannot accept draw".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn resign(&mut self, player_id: &str) -> Result<ResignResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.resign(player_id) {
                Ok(ResignResult {
                    game_id,
                    resigned_by: game.resigned_by.clone(),
                })
            } else {
                Err("Cannot resign".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn claim_threefold_repetition(&mut self, player_id: &str) -> Result<ClaimDrawResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.claim_threefold_repetition(player_id) {
                Ok(ClaimDrawResult {
                    game_id,
                    claimed_draw_by: game.claimed_draw_by.clone(),
                    claimed_draw_reason: game.claimed_draw_reason.clone(),
                })
            } else {
                Err("Cannot claim threefold repetition".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn claim_fifty_moves(&mut self, player_id: &str) -> Result<ClaimDrawResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.claim_fifty_moves(player_id) {
                Ok(ClaimDrawResult {
                    game_id,
                    claimed_draw_by: game.claimed_draw_by.clone(),
                    claimed_draw_reason: game.claimed_draw_reason.clone(),
                })
            } else {
                Err("Cannot claim fifty-move rule".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn request_rematch(&mut self, player_id: &str) -> Result<RematchResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            
            if !game.is_game_over() {
                return Err("Game is not over yet".to_string());
            }
            
            game.game_id.clone()
        };
        if let Some(game) = self.games.get_mut(&game_id) {
            if Some(player_id) == game.rematch_requested_by.as_deref() {
                return Ok(RematchResult {
                    game_id: game_id.clone(),
                    rematch_requested_by: game.rematch_requested_by.clone(),
                    rematch_accepted_by: game.rematch_accepted_by.clone(),
                    both_accepted: false,
                });
            }
            
            if let Some(ref req_by) = game.rematch_requested_by {
                if req_by != player_id {
                    game.rematch_accepted_by = Some(player_id.to_string());
                    return Ok(RematchResult {
                        game_id: game_id.clone(),
                        rematch_requested_by: game.rematch_requested_by.clone(),
                        rematch_accepted_by: game.rematch_accepted_by.clone(),
                        both_accepted: true,
                    });
                }
            }
            
            game.rematch_requested_by = Some(player_id.to_string());
            Ok(RematchResult {
                game_id: game_id.clone(),
                rematch_requested_by: game.rematch_requested_by.clone(),
                rematch_accepted_by: game.rematch_accepted_by.clone(),
                both_accepted: false,
            })
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn accept_rematch(&mut self, player_id: &str) -> Result<RematchResult, String> {
        let game_id = {
            let game = match self.get_player_game(player_id) {
                Some(g) => g,
                None => return Err("Player not in a game".to_string()),
            };
            game.game_id.clone()
        };
        if let Some(game) = self.games.get_mut(&game_id) {
            if game.rematch_requested_by.is_none() {
                return Err("No rematch request to accept".to_string());
            }
            
            if Some(player_id) == game.rematch_requested_by.as_deref() {
                return Err("Cannot accept your own rematch request".to_string());
            }
            
            if Some(player_id) == game.rematch_accepted_by.as_deref() {
                return Ok(RematchResult {
                    game_id: game_id.clone(),
                    rematch_requested_by: game.rematch_requested_by.clone(),
                    rematch_accepted_by: game.rematch_accepted_by.clone(),
                    both_accepted: false,
                });
            }
            
            game.rematch_accepted_by = Some(player_id.to_string());
            
            if game.rematch_requested_by.is_some() && game.rematch_accepted_by.is_some() {
                if game.colors_assigned || game.game_started {
                    game.reset_game_for_rematch();
                    game.assign_colors_randomly();
                }
                
                return Ok(RematchResult {
                    game_id: game_id.clone(),
                    rematch_requested_by: game.rematch_requested_by.clone(),
                    rematch_accepted_by: game.rematch_accepted_by.clone(),
                    both_accepted: true,
                });
            }
            
            Ok(RematchResult {
                game_id: game_id.clone(),
                rematch_requested_by: game.rematch_requested_by.clone(),
                rematch_accepted_by: game.rematch_accepted_by.clone(),
                both_accepted: false,
            })
        } else {
            Err("Game not found".to_string())
        }
    }
    
    pub fn reject_rematch(&mut self, player_id: &str) -> Result<RejectRematchResult, String> {
        let game = match self.get_player_game(player_id) {
            Some(g) => g,
            None => return Err("Player not in a game".to_string()),
        };
        
        let is_creator = game.player1.as_deref() == Some(player_id);
        
        Ok(RejectRematchResult {
            game_id: game.game_id.clone(),
            is_creator,
        })
    }
    
    pub fn kick_players_to_lobby(&mut self, game_id: &str) -> Vec<String> {
        if let Some(game) = self.games.get(game_id) {
            let mut players_to_kick = Vec::new();
            
            if game.colors_assigned {
                if let Some(ref wp) = game.white_player {
                    players_to_kick.push(wp.clone());
                }
                if let Some(ref bp) = game.black_player {
                    players_to_kick.push(bp.clone());
                }
            } else {
                if let Some(ref p1) = game.player1 {
                    players_to_kick.push(p1.clone());
                }
                if let Some(ref p2) = game.player2 {
                    players_to_kick.push(p2.clone());
                }
            }
            
            for player_id in &players_to_kick {
                self.player_to_game.remove(player_id);
            }
            
            // Kick all spectators
            if let Some(spectators) = self.spectators.remove(game_id) {
                for spectator_id in &spectators {
                    self.spectator_to_game.remove(spectator_id);
                    players_to_kick.push(spectator_id.clone());
                }
            }
            
            self.games.remove(game_id);
            players_to_kick
        } else {
            Vec::new()
        }
    }

    pub fn forfeit_game_for_player(&mut self, player_id: &str) -> Option<ForfeitResult> {
        self.lobby_queue.retain(|id| id != player_id);

        let game = match self.get_player_game(player_id) {
            Some(g) => g,
            None => return None,
        };
        
        let game_id = game.game_id.clone();
        let room_id = game.room_id.clone();
        let is_host = game.player1.as_deref() == Some(player_id);
        let _is_in_game_or_rematch = game.game_started || game.is_game_over() || 
                                     game.rematch_requested_by.is_some() || 
                                     game.rematch_accepted_by.is_some();
        
        // Get opponent - handle both cases: colors assigned or not
        let opponent_id = if game.colors_assigned {
            if Some(player_id) == game.white_player.as_deref() {
                game.black_player.clone()
            } else {
                game.white_player.clone()
            }
        } else {
            // Colors not assigned yet, use player1/player2
            if Some(player_id) == game.player1.as_deref() {
                game.player2.clone()
            } else {
                game.player1.clone()
            }
        };
        
        // If host disconnects in any state, kick everyone and delete room
        if is_host {
            let _ = self.kick_players_to_lobby(&game_id);
            return None;
        }
        
        // Non-host disconnecting - always reset game and keep host in room
        if let Some(game) = self.games.get_mut(&game_id) {
            // Save host before any changes
            let host = game.player1.clone();
            
            // Clear player fields based on whether colors are assigned
            if game.colors_assigned {
                if game.white_player.as_deref() == Some(player_id) {
                    game.white_player = None;
                }
                if game.black_player.as_deref() == Some(player_id) {
                    game.black_player = None;
                }
            } else {
                // Colors not assigned yet - clear player2 (non-host) or player1 (host)
                if game.player2.as_deref() == Some(player_id) {
                    game.player2 = None;
                }
                if game.player1.as_deref() == Some(player_id) {
                    game.player1 = None;
                }
            }
            
            // Check if game is now empty (zero players)
            let is_game_empty = game.player1.is_none() && game.player2.is_none();
            
            if is_game_empty {
                // Game has zero players - delete it
                // Remove any remaining player mappings
                let players_to_remove: Vec<String> = self
                    .player_to_game
                    .iter()
                    .filter(|(_, gid)| *gid == &game_id)
                    .map(|(pid, _)| pid.clone())
                    .collect();
                for pid in players_to_remove {
                    self.player_to_game.remove(&pid);
                }
                
                // Delete the game
                self.games.remove(&game_id);
                // Return None to indicate game was deleted
                return None;
            } else if let Some(ref host_id) = host {
                // Host is still in the game, reset game state so they can start a new game
                // This happens regardless of game state - host should never get stuck
                // Reset game state completely
                game.reset_game_for_rematch();
                // Restore host after reset
                game.player1 = host.clone();
                // Ensure host is still in player_to_game mapping
                self.player_to_game.insert(host_id.clone(), game_id.clone());
                // Return None to indicate game was reset, not forfeited
                return None;
            }
        }
        
        // Let the departing player re-enter the lobby/queue
        self.player_to_game.remove(player_id);
        
        Some(ForfeitResult {
            game_id,
            room_id,
            opponent_id: opponent_id.clone(),
            resigned_by: None, // Game was reset, not forfeited
            winner_id: opponent_id,
        })
    }

    pub fn remove_player(&mut self, player_id: &str) {
        self.lobby_queue.retain(|id| id != player_id);

        if let Some(game_id) = self.player_to_game.remove(player_id) {
            // Get game info before mutating
            let (is_host, _is_in_game_or_rematch) = {
                if let Some(game) = self.games.get(&game_id) {
                    let is_host = game.player1.as_deref() == Some(player_id);
                    let is_in_game_or_rematch = game.game_started || game.is_game_over() || 
                                                 game.rematch_requested_by.is_some() || 
                                                 game.rematch_accepted_by.is_some();
                    (is_host, is_in_game_or_rematch)
                } else {
                    // Game doesn't exist, nothing to do
                    return;
                }
            };
            
            // If host disconnects, kick everyone and delete room (regardless of game state)
            if is_host {
                let _ = self.kick_players_to_lobby(&game_id);
                return;
            }
            
            // Non-host disconnecting - always reset game and keep host in room
            if let Some(game) = self.games.get_mut(&game_id) {
                // Save host before any changes
                let host = game.player1.clone();
                
                // Clear player fields based on whether colors are assigned
                if game.colors_assigned {
                    // When colors are assigned, clear the color-specific fields
                    if Some(player_id) == game.white_player.as_deref() {
                        game.white_player = None;
                    }
                    if Some(player_id) == game.black_player.as_deref() {
                        game.black_player = None;
                    }
                    // Also clear player2 if the disconnected player was player2
                    // (player1 is the host and should remain)
                    if Some(player_id) == game.player2.as_deref() {
                        game.player2 = None;
                    }
                } else {
                    // Colors not assigned yet - clear player2 (non-host) or player1 (host)
                    if Some(player_id) == game.player2.as_deref() {
                        game.player2 = None;
                    }
                    if Some(player_id) == game.player1.as_deref() {
                        game.player1 = None;
                    }
                }
                
                // Check if game is now empty (zero players)
                // Game is empty only if player1 (host) is None - if host exists, game should continue
                let is_game_empty = game.player1.is_none();
                
                if is_game_empty {
                    // Game has zero players - delete it
                    // Remove any remaining player mappings
                    let players_to_remove: Vec<String> = self
                        .player_to_game
                        .iter()
                        .filter(|(_, gid)| *gid == &game_id)
                        .map(|(pid, _)| pid.clone())
                        .collect();
                    for pid in players_to_remove {
                        self.player_to_game.remove(&pid);
                    }
                    
                    // Delete the game
                    self.games.remove(&game_id);
                } else if let Some(ref host_id) = host {
                    // Host is still in the game, reset game state so they can start a new game
                    // This happens regardless of game state - host should never get stuck
                    // Reset game state completely
                    game.reset_game_for_rematch();
                    // Restore host after reset
                    game.player1 = host.clone();
                    game.player2 = None; // Explicitly clear player2 since reset_game_for_rematch doesn't clear it
                    // Ensure host is still in player_to_game mapping (critical for host to not get stuck)
                    self.player_to_game.insert(host_id.clone(), game_id.clone());
                }
            }
        }
    }
}

