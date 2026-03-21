use crate::game_manager::RoomResult;
use crate::messages::server::{self, ServerMessage};
use crate::chess_game::TimeControl;
use super::base::HandlerContext;
use super::helpers;

pub struct RoomHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> RoomHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn create_room(&self, password: Option<String>, allow_spectators: Option<bool>, time_control: Option<String>) -> Result<(), axum::Error> {
        tracing::info!("handle_create_room called with password: {:?}, allow_spectators: {:?}, time_control: {:?}", password, allow_spectators, time_control);
        
        let password = password.and_then(|p| {
            let p = p.trim().to_string();
            if p.len() > 20 {
                return None;
            }
            if p.is_empty() {
                None
            } else {
                Some(p)
            }
        });

        let allow_spectators = allow_spectators.unwrap_or(true); // Default to true if not specified

        // Parse time_control string to enum, default to Classical if invalid/missing
        let time_control_enum = time_control
            .as_ref()
            .and_then(|tc| match tc.as_str() {
                "bullet" => Some(TimeControl::Bullet),
                "blitz" => Some(TimeControl::Blitz),
                "rapid" => Some(TimeControl::Rapid),
                "classical" => Some(TimeControl::Classical),
                _ => None,
            })
            .unwrap_or(TimeControl::Classical);

        let (result, affected_spectators) = {
            let mut gm = self.ctx.game_manager.write().await;
            let result = gm.create_room(self.ctx.player_id.clone(), password.clone(), allow_spectators, time_control_enum);
            let affected = gm.cleanup_empty_games();
            (result, affected)
        };
        
        helpers::notify_spectators_of_game_deletion(
            &affected_spectators,
            self.ctx.game_manager.clone(),
            self.ctx.state_manager.clone(),
        ).await?;
        
        match result {
            RoomResult::Success { game_id, room_id, .. } => {
                tracing::info!("Room created successfully: game_id={}, room_id={}", game_id, room_id);
                server::send_json(&self.ctx.sender, &ServerMessage::RoomCreated {
                    game_id: game_id.clone(),
                    room_id: room_id.clone(),
                    white_player: None,
                    black_player: None,
                    color: None,
                    game_started: false,
                })?;
                
                helpers::send_game_state_to_player(
                    &self.ctx.player_id,
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                    self.ctx.sender.clone(),
                ).await?;
                Ok(())
            }
            RoomResult::Error { error } => {
                tracing::error!("Failed to create room: {}", error);
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: error })
            }
        }
    }

    pub async fn join_room(&self, room_id: String, password: Option<String>) -> Result<(), axum::Error> {
        let password = password.map(|p| p.trim().to_string());
        
        let (result, affected_spectators) = {
            let mut gm = self.ctx.game_manager.write().await;
            let result = gm.join_room(self.ctx.player_id.clone(), room_id.clone(), password);
            let affected = gm.cleanup_empty_games();
            (result, affected)
        };
        
        helpers::notify_spectators_of_game_deletion(
            &affected_spectators,
            self.ctx.game_manager.clone(),
            self.ctx.state_manager.clone(),
        ).await?;
        
        match result {
            RoomResult::Success { game_id, room_id, .. } => {
                server::send_json(&self.ctx.sender, &ServerMessage::RoomJoined {
                    game_id: game_id.clone(),
                    room_id: room_id.clone(),
                    white_player: None,
                    black_player: None,
                    color: None,
                    game_started: false,
                })?;
                
                helpers::send_game_state_to_all(
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await?;
                
                let (opponent_name, opponent_color) = {
                    let gm = self.ctx.game_manager.read().await;
                    let sm = self.ctx.state_manager.read().await;
                    if let Some(game) = gm.get_game(&game_id) {
                        let joined_player_id = if Some(&self.ctx.player_id) == game.player1.as_ref() {
                            game.player1.as_ref()
                        } else if Some(&self.ctx.player_id) == game.player2.as_ref() {
                            game.player2.as_ref()
                        } else {
                            None
                        };
                        
                        let name = if let Some(id) = joined_player_id {
                            sm.get_player_name(id).await
                        } else {
                            None
                        };
                        let color = game.get_player_color(&self.ctx.player_id);
                        (name, color)
                    } else {
                        (None, None)
                    }
                };
                
                let msg = ServerMessage::OpponentJoined {
                    game_id: game_id.clone(),
                    room_id: room_id.clone(),
                    opponent_name: opponent_name.clone(),
                    opponent_color: opponent_color.clone(),
                    message: if let Some(name) = &opponent_name {
                        Some(format!("{} joined the game!", name))
                    } else {
                        Some("A player joined the game!".to_string())
                    },
                };
                helpers::send_message_to_all_in_game(
                    &game_id,
                    msg,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                ).await;
                
                Ok(())
            }
            RoomResult::Error { error } => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: error })
            }
        }
    }

    pub async fn spectate_room(&self, room_id: String, password: Option<String>) -> Result<(), axum::Error> {
        let password = password.map(|p| p.trim().to_string());
        
        let (result, affected_spectators) = {
            let mut gm = self.ctx.game_manager.write().await;
            let result = gm.spectate_room(self.ctx.player_id.clone(), room_id.clone(), password);
            let affected = gm.cleanup_empty_games();
            (result, affected)
        };
        
        helpers::notify_spectators_of_game_deletion(
            &affected_spectators,
            self.ctx.game_manager.clone(),
            self.ctx.state_manager.clone(),
        ).await?;
        
        match result {
            RoomResult::Success { game_id, room_id, .. } => {
                server::send_json(&self.ctx.sender, &ServerMessage::RoomSpectated {
                    game_id: game_id.clone(),
                    room_id: room_id.clone(),
                    white_player: None,
                    black_player: None,
                    color: None,
                    game_started: false,
                })?;
                
                helpers::send_game_state_to_player(
                    &self.ctx.player_id,
                    &game_id,
                    self.ctx.game_manager.clone(),
                    self.ctx.state_manager.clone(),
                    self.ctx.sender.clone(),
                ).await?;
                Ok(())
            }
            RoomResult::Error { error } => {
                server::send_json(&self.ctx.sender, &ServerMessage::Error { message: error })
            }
        }
    }
}

