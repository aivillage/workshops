use crate::messages::server::{self, ServerMessage};
use super::base::HandlerContext;
use super::helpers;

pub struct DrawHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> DrawHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn offer_draw(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot offer draws".to_string(),
            });
        }
        
        match gm.offer_draw(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let draw_offered_by = result.draw_offered_by.clone();
                drop(gm);
                
                let offered_by_name = {
                    let sm = self.ctx.state_manager.read().await;
                    if let Some(id) = draw_offered_by.as_ref() {
                        sm.get_player_name(id).await
                    } else {
                        None
                    }
                };
                
                let msg = ServerMessage::DrawOffered {
                    offered_by: draw_offered_by.clone().unwrap_or_default(),
                    offered_by_name: offered_by_name.clone(),
                    message: if let Some(name) = &offered_by_name {
                        Some(format!("{} offered a draw", name))
                    } else {
                        Some("A player offered a draw".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn accept_draw(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot accept draws".to_string(),
            });
        }
        
        match gm.accept_draw(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                drop(gm);
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let accepted_by_name = {
                    let sm = self.ctx.state_manager.read().await;
                    sm.get_player_name(&self.ctx.player_id).await
                };
                
                let msg = ServerMessage::DrawAccepted {
                    accepted_by: Some(self.ctx.player_id.clone()),
                    accepted_by_name: accepted_by_name.clone(),
                    message: if let Some(name) = &accepted_by_name {
                        Some(format!("{} accepted the draw. Game ends in a draw by agreement.", name))
                    } else {
                        Some("Draw accepted. Game ends in a draw by agreement.".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                server::send_json(&self.ctx.sender, &ServerMessage::DrawAccepted {
                    accepted_by: Some(self.ctx.player_id.clone()),
                    accepted_by_name: accepted_by_name,
                    message: Some("You accepted the draw. Game ends in a draw by agreement.".to_string()),
                })
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn resign(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot resign".to_string(),
            });
        }
        
        match gm.resign(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let resigned_by = result.resigned_by.clone().unwrap_or_default();
                drop(gm);
                
                let (resigned_by_name, winner_name) = {
                    let gm = self.ctx.game_manager.read().await;
                    let sm = self.ctx.state_manager.read().await;
                    if let Some(game) = gm.get_game(&game_id) {
                        let resigned_name = sm.get_player_name(&resigned_by).await;
                        
                        let winner_id = if Some(&resigned_by) == game.white_player.as_ref() {
                            game.black_player.clone()
                        } else {
                            game.white_player.clone()
                        };
                        let winner = if let Some(id) = winner_id {
                            sm.get_player_name(&id).await
                        } else {
                            None
                        };
                        (resigned_name, winner)
                    } else {
                        (None, None)
                    }
                };
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let msg = ServerMessage::Resignation {
                    resigned_by: resigned_by.clone(),
                    resigned_by_name: resigned_by_name.clone(),
                    winner_name: winner_name.clone(),
                    message: if let (Some(resigned), Some(winner)) = (&resigned_by_name, &winner_name) {
                        Some(format!("{} resigned. {} wins the game!", resigned, winner))
                    } else if let Some(resigned) = &resigned_by_name {
                        Some(format!("{} resigned. Game over.", resigned))
                    } else {
                        Some("A player resigned. Game over.".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                let personal_msg = if let Some(_name) = &resigned_by_name {
                    "You resigned. Game over.".to_string()
                } else {
                    "You resigned. Game over.".to_string()
                };
                
                server::send_json(&self.ctx.sender, &ServerMessage::Resignation {
                    resigned_by,
                    resigned_by_name,
                    winner_name,
                    message: Some(personal_msg),
                })
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn claim_threefold(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot claim draws".to_string(),
            });
        }
        
        match gm.claim_threefold_repetition(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let claimed_by = result.claimed_draw_by.clone().unwrap_or_default();
                let reason = result.claimed_draw_reason.clone().unwrap_or_default();
                drop(gm);
                
                let claimed_by_name = {
                    let sm = self.ctx.state_manager.read().await;
                    sm.get_player_name(&claimed_by).await
                };
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let msg = ServerMessage::DrawClaimed {
                    claimed_by: claimed_by.clone(),
                    claimed_by_name: claimed_by_name.clone(),
                    reason: reason.clone(),
                    message: if let Some(name) = &claimed_by_name {
                        Some(format!("{} claimed a draw by threefold repetition. The same position occurred three times. Game ends in a draw.", name))
                    } else {
                        Some("A player claimed a draw by threefold repetition. The same position occurred three times. Game ends in a draw.".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                server::send_json(&self.ctx.sender, &ServerMessage::DrawClaimed {
                    claimed_by,
                    claimed_by_name,
                    reason,
                    message: Some("You claimed a draw by threefold repetition. The same position occurred three times. Game ends in a draw.".to_string()),
                })
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn claim_fifty_moves(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot claim draws".to_string(),
            });
        }
        
        match gm.claim_fifty_moves(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let claimed_by = result.claimed_draw_by.clone().unwrap_or_default();
                let reason = result.claimed_draw_reason.clone().unwrap_or_default();
                drop(gm);
                
                let claimed_by_name = {
                    let sm = self.ctx.state_manager.read().await;
                    sm.get_player_name(&claimed_by).await
                };
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let msg = ServerMessage::DrawClaimed {
                    claimed_by: claimed_by.clone(),
                    claimed_by_name: claimed_by_name.clone(),
                    reason: reason.clone(),
                    message: if let Some(name) = &claimed_by_name {
                        Some(format!("{} claimed a draw by the fifty-move rule. No pawn move or capture occurred in the last 50 moves. Game ends in a draw.", name))
                    } else {
                        Some("A player claimed a draw by the fifty-move rule. No pawn move or capture occurred in the last 50 moves. Game ends in a draw.".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                server::send_json(&self.ctx.sender, &ServerMessage::DrawClaimed {
                    claimed_by,
                    claimed_by_name,
                    reason,
                    message: Some("You claimed a draw by the fifty-move rule. No pawn move or capture occurred in the last 50 moves. Game ends in a draw.".to_string()),
                })
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }
}

