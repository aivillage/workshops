use crate::handlers::base::HandlerContext;
use crate::messages::server::{self, ServerMessage};
use crate::handlers::helpers;

pub struct PlayerHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> PlayerHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn set_player_name(&self, name: String) -> Result<(), axum::Error> {
        let new_name = name.trim();
        if new_name.is_empty() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Name cannot be empty".to_string(),
            });
        }
        
        let new_name = if new_name.len() > 20 {
            &new_name[..20]
        } else {
            new_name
        };
        
        let sm = self.ctx.state_manager.write().await;
        sm.set_player_name(self.ctx.player_id.clone(), new_name.to_string()).await;
        drop(sm);
        
        let game_id = {
            let gm = self.ctx.game_manager.read().await;
            gm.get_player_game(&self.ctx.player_id).map(|g| g.game_id.clone())
        };
        
        if let Some(game_id) = game_id {
            helpers::send_game_state_to_all(
                &game_id,
                self.ctx.game_manager.clone(),
                self.ctx.state_manager.clone(),
            ).await?;
        }
        
        server::send_json(&self.ctx.sender, &ServerMessage::Error {
            message: format!("Player name updated to {}", new_name),
        })
    }

    pub async fn leave_game(&self) -> Result<(), axum::Error> {
        let (was_creator, game_id, game_started, host_id) = {
            let gm = self.ctx.game_manager.read().await;
            if let Some(game) = gm.get_player_game(&self.ctx.player_id) {
                let was_creator = game.player1.as_deref() == Some(&self.ctx.player_id);
                let host_id = game.player1.clone();
                (was_creator, Some(game.game_id.clone()), game.game_started, host_id)
            } else {
                (false, None, false, None)
            }
        };
        
        let leave_info = {
            let mut gm = self.ctx.game_manager.write().await;
            gm.forfeit_game_for_player(&self.ctx.player_id)
        };
        
        if leave_info.is_none() && game_id.is_some() && !was_creator {
            let game_id = game_id.unwrap();
            let host_still_in_game = {
                let gm = self.ctx.game_manager.read().await;
                if let Some(game) = gm.get_game(&game_id) {
                    host_id.as_ref()
                        .map(|h| game.player1.as_deref() == Some(h.as_str()))
                        .unwrap_or(false)
                } else {
                    false
                }
            };
            
            if host_still_in_game {
                let left_player_name = {
                    let sm = self.ctx.state_manager.read().await;
                    sm.get_player_name(&self.ctx.player_id).await
                };
                
                if let Some(ref host_id) = host_id {
                    if let Some(host_sender) = self.ctx.state_manager.read().await.get_connection(host_id).await {
                        let msg = ServerMessage::OpponentLeft {
                            opponent_name: left_player_name.clone(),
                            message: if let Some(name) = &left_player_name {
                                Some(format!("{} left the game. Waiting for a new opponent to join...", name))
                            } else {
                                Some("Opponent left the game. Waiting for a new opponent to join...".to_string())
                            },
                        };
                        helpers::send_message_to_all_in_game(
                            &game_id,
                            msg,
                            self.ctx.game_manager.clone(),
                            self.ctx.state_manager.clone(),
                        ).await;
                        
                        let _ = helpers::send_game_state_to_player(
                            host_id,
                            &game_id,
                            self.ctx.game_manager.clone(),
                            self.ctx.state_manager.clone(),
                            host_sender,
                        ).await;
                    }
                }
                
                let spectator_ids = {
                    let gm = self.ctx.game_manager.read().await;
                    gm.get_spectators(&game_id)
                };
                
                for spectator_id in spectator_ids {
                    if let Some(spectator_sender) = self.ctx.state_manager.read().await.get_connection(&spectator_id).await {
                        let msg = ServerMessage::OpponentLeft {
                            opponent_name: left_player_name.clone(),
                            message: if let Some(name) = &left_player_name {
                                Some(format!("{} left the game.", name))
                            } else {
                                Some("A player left the game.".to_string())
                            },
                        };
                        helpers::send_message_to_all_in_game(
                            &game_id,
                            msg,
                            self.ctx.game_manager.clone(),
                            self.ctx.state_manager.clone(),
                        ).await;
                        
                        let _ = helpers::send_game_state_to_player(
                            &spectator_id,
                            &game_id,
                            self.ctx.game_manager.clone(),
                            self.ctx.state_manager.clone(),
                            spectator_sender,
                        ).await;
                    }
                }
            }
        } else if let Some(leave_info) = leave_info {
            let game_id = leave_info.game_id.clone();
            let opponent_id = leave_info.opponent_id.clone();
            
            if game_started {
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let disconnected_player_name = {
                    let sm = self.ctx.state_manager.read().await;
                    sm.get_player_name(&self.ctx.player_id).await
                };
                
                if let Some(ref opponent_id) = opponent_id {
                    if let Some(opponent_sender) = self.ctx.state_manager.read().await.get_connection(opponent_id).await {
                        let msg = ServerMessage::OpponentDisconnected {
                            opponent_name: disconnected_player_name.clone(),
                            message: if let Some(name) = &disconnected_player_name {
                                Some(format!("{} disconnected during the game. Game ended. Returning to lobby...", name))
                            } else {
                                Some("Opponent disconnected during the game. Game ended. Returning to lobby...".to_string())
                            },
                        };
                        let json = serde_json::to_string(&msg).unwrap();
                        let _ = opponent_sender.send(axum::extract::ws::Message::Text(json));
                    }
                }
            } else {
                if was_creator {
                    if let Some(ref opponent_id) = opponent_id {
                        let mut gm = self.ctx.game_manager.write().await;
                        gm.remove_player(opponent_id);
                        drop(gm);
                        
                        if let Some(joiner_sender) = self.ctx.state_manager.read().await.get_connection(opponent_id).await {
                            let kicked_msg = ServerMessage::KickedToLobby {
                                message: "Host left the game. Returning to lobby...".to_string(),
                            };
                            let json = serde_json::to_string(&kicked_msg).unwrap();
                            let _ = joiner_sender.send(axum::extract::ws::Message::Text(json));
                            
                            let lobby_status = {
                                let mut gm = self.ctx.game_manager.write().await;
                                gm.get_lobby_status(opponent_id)
                            };
                            let lobby_msg = ServerMessage::LobbyStatus { status: lobby_status };
                            let json = serde_json::to_string(&lobby_msg).unwrap();
                            let _ = joiner_sender.send(axum::extract::ws::Message::Text(json));
                        }
                    }
                }
            }
        }
        
        let lobby_status = {
            let mut gm = self.ctx.game_manager.write().await;
            gm.get_lobby_status(&self.ctx.player_id)
        };
        
        server::send_json(&self.ctx.sender, &ServerMessage::LobbyStatus { status: lobby_status })
    }
}

