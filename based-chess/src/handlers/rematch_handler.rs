use crate::messages::server::{self, ServerMessage};
use super::base::HandlerContext;
use super::helpers;

pub struct RematchHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> RematchHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn request_rematch(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot request rematches".to_string(),
            });
        }
        
        match gm.request_rematch(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let both_accepted = result.both_accepted;
                drop(gm);
                
                if both_accepted {
                    let mut gm = self.ctx.game_manager.write().await;
                    if let Some(game) = gm.get_game_mut(&game_id) {
                        game.reset_game_for_rematch();
                        game.assign_colors_randomly();
                        
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
                        drop(gm);
                        
                        // Send rematch_started messages using helper
                        let gm_read = self.ctx.game_manager.read().await;
                        let sm_read = self.ctx.state_manager.read().await;
                        if let Some(game) = gm_read.get_game(&game_id) {
                            let black_name = black_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                            let white_name = white_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                            
                            if let Some(white_id) = &game.white_player {
                                if let Some(white_sender) = sm_read.get_connection(white_id).await {
                                    let msg = ServerMessage::RematchStarted {
                                        your_color: "white".to_string(),
                                        white_player_name: white_player_name.clone(),
                                        black_player_name: black_player_name.clone(),
                                        message: Some(format!("Rematch started! You are playing as White against {}. Press 'Start Game' when ready.", black_name)),
                                    };
                                    let json = serde_json::to_string(&msg).unwrap();
                                    let _ = white_sender.send(axum::extract::ws::Message::Text(json));
                                }
                            }
                            if let Some(black_id) = &game.black_player {
                                if let Some(black_sender) = sm_read.get_connection(black_id).await {
                                    let msg = ServerMessage::RematchStarted {
                                        your_color: "black".to_string(),
                                        white_player_name: white_player_name.clone(),
                                        black_player_name: black_player_name.clone(),
                                        message: Some(format!("Rematch started! You are playing as Black against {}. Press 'Start Game' when ready.", white_name)),
                                    };
                                    let json = serde_json::to_string(&msg).unwrap();
                                    let _ = black_sender.send(axum::extract::ws::Message::Text(json));
                                }
                            }
                            
                            let spectator_ids = gm_read.get_spectators(&game_id);
                            for spectator_id in spectator_ids {
                                if let Some(spectator_sender) = sm_read.get_connection(&spectator_id).await {
                                    let spectator_msg = ServerMessage::RematchStarted {
                                        your_color: "spectator".to_string(),
                                        white_player_name: white_player_name.clone(),
                                        black_player_name: black_player_name.clone(),
                                        message: Some(format!("Rematch started! Waiting for {} and {} to begin the game...", white_name, black_name)),
                                    };
                                    let json = serde_json::to_string(&spectator_msg).unwrap();
                                    let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
                                }
                            }
                        }
                        drop(gm_read);
                        drop(sm_read);
                        
                        helpers::send_game_state_to_all(
                            &game_id,
                            self.ctx.game_manager.clone(),
                            self.ctx.state_manager.clone(),
                        ).await?;
                    }
                } else {
                    let requested_by_name = {
                        let sm = self.ctx.state_manager.read().await;
                        sm.get_player_name(&self.ctx.player_id).await
                    };
                    
                    let msg = ServerMessage::RematchRequested {
                        requested_by: Some(self.ctx.player_id.clone()),
                        requested_by_name: requested_by_name.clone(),
                        message: if let Some(name) = &requested_by_name {
                            Some(format!("{} requested a rematch. Waiting for opponent to accept...", name))
                        } else {
                            Some("A player requested a rematch. Waiting for opponent to accept...".to_string())
                        },
                    };
                    helpers::send_message_to_all_in_game(
                        &game_id,
                        msg,
                        self.ctx.game_manager.clone(),
                        self.ctx.state_manager.clone(),
                    ).await;
                    
                    // Start timeout task
                    let game_id_clone = game_id.clone();
                    let gm_clone = self.ctx.game_manager.clone();
                    let sm_clone = self.ctx.state_manager.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                        let mut gm = gm_clone.write().await;
                        if let Some(game) = gm.get_game_mut(&game_id_clone) {
                            if game.rematch_requested_by.is_some() && game.rematch_accepted_by.is_some() {
                                return;
                            }
                            
                            let requester_id = game.rematch_requested_by.clone();
                            let accepter_id = game.rematch_accepted_by.clone();
                            game.rematch_requested_by = None;
                            game.rematch_accepted_by = None;
                            drop(gm);
                            
                            let sm = sm_clone.read().await;
                            if let Some(ref requester_id) = requester_id {
                                if let Some(requester_sender) = sm.get_connection(requester_id).await {
                                    let msg = ServerMessage::RematchTimeout {
                                        message: "Rematch request timed out".to_string(),
                                    };
                                    let json = serde_json::to_string(&msg).unwrap();
                                    let _ = requester_sender.send(axum::extract::ws::Message::Text(json));
                                }
                            }
                            if let Some(ref accepter_id) = accepter_id {
                                if let Some(accepter_sender) = sm.get_connection(accepter_id).await {
                                    let msg = ServerMessage::RematchTimeout {
                                        message: "Rematch request timed out".to_string(),
                                    };
                                    let json = serde_json::to_string(&msg).unwrap();
                                    let _ = accepter_sender.send(axum::extract::ws::Message::Text(json));
                                }
                            }
                            
                            let gm = gm_clone.read().await;
                            if let Some(game) = gm.get_game(&game_id_clone) {
                                let player_ids: Vec<String> = vec![
                                    game.player1.clone(),
                                    game.player2.clone(),
                                    game.white_player.clone(),
                                    game.black_player.clone(),
                                ]
                                .into_iter()
                                .flatten()
                                .collect();
                                drop(gm);
                                
                                for player_id in player_ids {
                                    if let Some(sender) = sm.get_connection(&player_id).await {
                                        let _ = helpers::send_game_state_to_player(
                                            &player_id,
                                            &game_id_clone,
                                            gm_clone.clone(),
                                            sm_clone.clone(),
                                            sender,
                                        ).await;
                                    }
                                }
                            }
                        }
                    });
                    
                    helpers::send_game_state_to_all(
                        &game_id,
                        self.ctx.game_manager.clone(),
                        self.ctx.state_manager.clone(),
                    ).await?;
                }
                Ok(())
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }

    pub async fn accept_rematch(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot accept rematches".to_string(),
            });
        }
        
        match gm.accept_rematch(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let both_accepted = result.both_accepted;
                drop(gm);
                
                if both_accepted {
                    let accepted_by_name = {
                        let sm = self.ctx.state_manager.read().await;
                        sm.get_player_name(&self.ctx.player_id).await
                    };
                    
                    let msg = ServerMessage::RematchAccepted {
                        accepted_by: Some(self.ctx.player_id.clone()),
                        accepted_by_name: accepted_by_name.clone(),
                        message: if let Some(name) = &accepted_by_name {
                            Some(format!("{} accepted the rematch request. Both players accepted! Starting rematch...", name))
                        } else {
                            Some("Rematch accepted. Both players accepted! Starting rematch...".to_string())
                        },
                    };
                    helpers::send_message_to_all_in_game(
                        &game_id,
                        msg,
                        self.ctx.game_manager.clone(),
                        self.ctx.state_manager.clone(),
                    ).await;
                    
                    let gm_read = self.ctx.game_manager.read().await;
                    if let Some(game) = gm_read.get_game(&game_id) {
                        let sm_read = self.ctx.state_manager.read().await;
                        let white_player_name = if let Some(id) = game.white_player.as_ref() {
                            sm_read.get_player_name(id).await
                        } else {
                            None
                        };
                        let black_player_name = if let Some(id) = game.black_player.as_ref() {
                            sm_read.get_player_name(id).await
                        } else {
                            None
                        };
                        let black_name = black_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        let white_name = white_player_name.clone().unwrap_or_else(|| "Opponent".to_string());
                        
                        if let Some(white_id) = &game.white_player {
                            if let Some(white_sender) = sm_read.get_connection(white_id).await {
                                let msg = ServerMessage::RematchStarted {
                                    your_color: "white".to_string(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Rematch started! You are playing as White against {}. Press 'Start Game' when ready.", black_name)),
                                };
                                let json = serde_json::to_string(&msg).unwrap();
                                let _ = white_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                        if let Some(black_id) = &game.black_player {
                            if let Some(black_sender) = sm_read.get_connection(black_id).await {
                                let msg = ServerMessage::RematchStarted {
                                    your_color: "black".to_string(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Rematch started! You are playing as Black against {}. Press 'Start Game' when ready.", white_name)),
                                };
                                let json = serde_json::to_string(&msg).unwrap();
                                let _ = black_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                        
                        let spectator_ids = gm_read.get_spectators(&game_id);
                        for spectator_id in spectator_ids {
                            if let Some(spectator_sender) = sm_read.get_connection(&spectator_id).await {
                                let spectator_msg = ServerMessage::RematchStarted {
                                    your_color: "spectator".to_string(),
                                    white_player_name: white_player_name.clone(),
                                    black_player_name: black_player_name.clone(),
                                    message: Some(format!("Rematch started! Waiting for {} and {} to begin the game...", white_name, black_name)),
                                };
                                let json = serde_json::to_string(&spectator_msg).unwrap();
                                let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
                            }
                        }
                    }
                    drop(gm_read);
                } else {
                    let accepted_by_name = {
                        let sm = self.ctx.state_manager.read().await;
                        sm.get_player_name(&self.ctx.player_id).await
                    };
                    
                    let msg = ServerMessage::RematchAccepted {
                        accepted_by: Some(self.ctx.player_id.clone()),
                        accepted_by_name: accepted_by_name.clone(),
                        message: if let Some(name) = &accepted_by_name {
                            Some(format!("{} accepted the rematch request. Waiting for both players to be ready...", name))
                        } else {
                            Some("Rematch accepted. Waiting for both players to be ready...".to_string())
                        },
                    };
                    helpers::send_message_to_all_in_game(
                        &game_id,
                        msg,
                        self.ctx.game_manager.clone(),
                        self.ctx.state_manager.clone(),
                    ).await;
                }
                
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

    pub async fn reject_rematch(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        
        if gm.get_spectated_game_for_player(&self.ctx.player_id).is_some() {
            return server::send_json(&self.ctx.sender, &ServerMessage::Error {
                message: "Spectators cannot reject rematches".to_string(),
            });
        }
        
        match gm.reject_rematch(&self.ctx.player_id) {
            Ok(result) => {
                let game_id = result.game_id.clone();
                let is_creator = result.is_creator;
                drop(gm);
                
                if is_creator {
                    let players_to_kick = {
                        let mut gm = self.ctx.game_manager.write().await;
                        gm.kick_players_to_lobby(&game_id)
                    };
                    
                    for pid in &players_to_kick {
                        if let Some(sender) = self.ctx.state_manager.read().await.get_connection(pid).await {
                            let msg = ServerMessage::RematchRejected {
                                message: "Host declined rematch. Returning to lobby...".to_string(),
                            };
                            let json = serde_json::to_string(&msg).unwrap();
                            let _ = sender.send(axum::extract::ws::Message::Text(json));
                            
                            let kicked_msg = ServerMessage::KickedToLobby {
                                message: "Host declined rematch. Returning to lobby...".to_string(),
                            };
                            let json = serde_json::to_string(&kicked_msg).unwrap();
                            let _ = sender.send(axum::extract::ws::Message::Text(json));
                            
                            let lobby_status = {
                                let mut gm = self.ctx.game_manager.write().await;
                                gm.get_lobby_status(pid)
                            };
                            let lobby_msg = ServerMessage::LobbyStatus { status: lobby_status };
                            let json = serde_json::to_string(&lobby_msg).unwrap();
                            let _ = sender.send(axum::extract::ws::Message::Text(json));
                        }
                    }
                } else {
                    let mut gm = self.ctx.game_manager.write().await;
                    if let Some(game) = gm.get_game_mut(&game_id) {
                        if game.player2.as_deref() == Some(&self.ctx.player_id) {
                            game.player2 = None;
                        } else if game.player1.as_deref() == Some(&self.ctx.player_id) {
                            game.player1 = None;
                        }
                        game.rematch_requested_by = None;
                        game.rematch_accepted_by = None;
                    }
                    gm.remove_player(&self.ctx.player_id);
                    
                    let creator_id = {
                        let gm = self.ctx.game_manager.read().await;
                        if let Some(game) = gm.get_game(&game_id) {
                            game.player1.clone().or(game.player2.clone())
                        } else {
                            None
                        }
                    };
                    
                    if let Some(creator_id) = creator_id {
                        if let Some(creator_sender) = self.ctx.state_manager.read().await.get_connection(&creator_id).await {
                            let msg = ServerMessage::RematchRejected {
                                message: "Opponent declined rematch. Waiting for new opponent...".to_string(),
                            };
                            let json = serde_json::to_string(&msg).unwrap();
                            let _ = creator_sender.send(axum::extract::ws::Message::Text(json));
                            
                            let _ = helpers::send_game_state_to_player(
                                &creator_id,
                                &game_id,
                                self.ctx.game_manager.clone(),
                                self.ctx.state_manager.clone(),
                                creator_sender,
                            ).await;
                        }
                    }
                    
                    let lobby_status = {
                        let mut gm = self.ctx.game_manager.write().await;
                        gm.get_lobby_status(&self.ctx.player_id)
                    };
                    server::send_json(&self.ctx.sender, &ServerMessage::RematchRejected {
                        message: "You declined rematch. Returning to lobby...".to_string(),
                    })?;
                    server::send_json(&self.ctx.sender, &ServerMessage::KickedToLobby {
                        message: "You declined rematch. Returning to lobby...".to_string(),
                    })?;
                    server::send_json(&self.ctx.sender, &ServerMessage::LobbyStatus { status: lobby_status })?;
                }
                
                Ok(())
            }
            Err(e) => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: e })
            }
        }
    }
}

