use crate::messages::server::{self, ServerMessage};
use super::base::HandlerContext;
use super::helpers;

pub struct GameHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> GameHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn make_move(&self, move_uci: String) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if let Some(spectated_game_id) = gm.get_spectated_game_for_player(&self.ctx.player_id) {
            if let Some(game) = gm.get_game(&spectated_game_id) {
                if !game.game_started {
                    return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                        message: "Game has not yet started".to_string(),
                    });
                }
            }
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot make moves".to_string(),
            });
        }
        
        let game = match gm.get_player_game(&self.ctx.player_id) {
            Some(g) => g,
            None => {
                return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Not in a game".to_string(),
                });
            }
        };
        
        let player_color = game.get_player_color(&self.ctx.player_id);
        if player_color.is_none() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Colors not assigned yet. Please wait for both players to be ready.".to_string(),
            });
        }
        
        let current_turn = game.get_turn();
        if player_color.as_deref() != Some(&current_turn) {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Not your turn".to_string(),
            });
        }
        
        let game_id = game.game_id.clone();
        if let Some(game) = gm.get_game_mut(&game_id) {
            if game.make_move(&move_uci) {
                drop(gm);
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                Ok(())
            } else {
                drop(gm);
                server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Invalid move".to_string(),
                })
            }
        } else {
            drop(gm);
            server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Game not found".to_string(),
            })
        }
    }

    pub async fn get_legal_moves(&self, square: String) -> Result<(), axum::Error> {
        let gm = self.ctx.game_manager.read().await;
        
        if let Some(spectated_game_id) = gm.get_spectated_game_for_player(&self.ctx.player_id) {
            if let Some(game) = gm.get_game(&spectated_game_id) {
                if !game.game_started {
                    return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                        message: "Game has not yet started".to_string(),
                    });
                }
            }
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot get legal moves".to_string(),
            });
        }
        
        let game = match gm.get_player_game(&self.ctx.player_id) {
            Some(g) => g,
            None => {
                return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Not in a game".to_string(),
                });
            }
        };
        
        let legal_moves = game.get_legal_moves_for_square(&square);
        let valid_squares: Vec<String> = legal_moves
            .iter()
            .map(|m| m.chars().skip(2).take(2).collect())
            .collect();
        
        server::send_json(&self.ctx.sender, &ServerMessage::LegalMoves {
            square,
            valid_squares,
            legal_moves,
        })
    }

    pub async fn player_ready(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if let Some(spectated_game_id) = gm.get_spectated_game_for_player(&self.ctx.player_id) {
            if let Some(game) = gm.get_game(&spectated_game_id) {
                if !game.game_started {
                    return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                        message: "Game has not yet started".to_string(),
                    });
                }
            }
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot set ready".to_string(),
            });
        }
        
        let game = match gm.get_player_game(&self.ctx.player_id) {
            Some(g) => g,
            None => {
                return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Not in a game".to_string(),
                });
            }
        };
        
        if !game.colors_assigned {
            if (game.player1.as_deref() == Some(&self.ctx.player_id) && game.player2.is_none()) ||
               (game.player2.as_deref() == Some(&self.ctx.player_id) && game.player1.is_none()) {
                return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Waiting for opponent to join".to_string(),
                });
            }
        } else {
            if (game.white_player.as_deref() == Some(&self.ctx.player_id) && game.black_player.is_none()) ||
               (game.black_player.as_deref() == Some(&self.ctx.player_id) && game.white_player.is_none()) {
                return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                    message: "Waiting for opponent to join".to_string(),
                });
            }
        }
        
        match gm.set_player_ready(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let game_started = result.game_started;
                drop(gm);
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                if game_started {
                    let gm = self.ctx.game_manager.read().await;
                    if let Some(game) = gm.get_game(&game_id) {
                        let sm = self.ctx.state_manager.read().await;
                        let white_player_name = if let Some(id) = game.white_player.as_ref() {
                            sm.get_player_name(id).await
                        } else {
                            None
                        };
                        let black_player_name = if let Some(id) = game.black_player.as_ref() {
                            sm.get_player_name(id).await
                        } else {
                            None
                        };
                        drop(sm);
                        
                        if let Some(white_id) = &game.white_player {
                            if let Some(white_sender) = self.ctx.state_manager.read().await.get_connection(white_id).await {
                                let opponent_name = black_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                                let white_msg = ServerMessage::GameStarted {
                                    your_color: "white".to_string(),
                                    white_player: game.white_player.clone(),
                                    black_player: game.black_player.clone(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Game started! You are playing as White against {}. Make your first move.", opponent_name)),
                                };
                                let json = serde_json::to_string(&white_msg).unwrap();
                                let _ = white_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                        if let Some(black_id) = &game.black_player {
                            if let Some(black_sender) = self.ctx.state_manager.read().await.get_connection(black_id).await {
                                let opponent_name = white_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                                let black_msg = ServerMessage::GameStarted {
                                    your_color: "black".to_string(),
                                    white_player: game.white_player.clone(),
                                    black_player: game.black_player.clone(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Game started! You are playing as Black against {}. Waiting for White's move.", opponent_name)),
                                };
                                let json = serde_json::to_string(&black_msg).unwrap();
                                let _ = black_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                        let spectator_ids = gm.get_spectators(&game_id);
                        let white_name = white_player_name.clone().unwrap_or_else(|| "White player".to_string());
                        let black_name = black_player_name.clone().unwrap_or_else(|| "Black player".to_string());
                        for spectator_id in spectator_ids {
                            if let Some(spectator_sender) = self.ctx.state_manager.read().await.get_connection(&spectator_id).await {
                                let spectator_msg = ServerMessage::GameStarted {
                                    your_color: "spectator".to_string(),
                                    white_player: game.white_player.clone(),
                                    black_player: game.black_player.clone(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Game started! {} (White) vs {} (Black).", white_name, black_name)),
                                };
                                let json = serde_json::to_string(&spectator_msg).unwrap();
                                let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn get_game_state(&self) -> Result<(), axum::Error> {
        let gm = self.ctx.game_manager.read().await;
        if let Some(game) = gm.get_player_game(&self.ctx.player_id) {
            let game_id = game.game_id.clone();
            drop(gm);
            helpers::send_game_state_to_player(
                &self.ctx.player_id,
                &game_id,
                self.ctx.game_manager.clone(),
                self.ctx.state_manager.clone(),
                self.ctx.sender.clone(),
            ).await
        } else if let Some(spectated_game_id) = gm.get_spectated_game_for_player(&self.ctx.player_id) {
            let game_id = spectated_game_id.clone();
            drop(gm);
            helpers::send_game_state_to_player(
                &self.ctx.player_id,
                &game_id,
                self.ctx.game_manager.clone(),
                self.ctx.state_manager.clone(),
                self.ctx.sender.clone(),
            ).await
        } else {
            server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Not in a game".to_string(),
            })
        }
    }
}

