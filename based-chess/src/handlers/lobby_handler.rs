use crate::game_manager::LobbyStatus;
use crate::messages::server::{self, ServerMessage};
use crate::utils::game_helpers;
use super::base::HandlerContext;
use super::helpers;

pub struct LobbyHandler<'a> {
    ctx: &'a HandlerContext,
}

impl<'a> LobbyHandler<'a> {
    pub fn new(ctx: &'a HandlerContext) -> Self {
        Self { ctx }
    }

    pub async fn get_games_list(&self) -> Result<(), axum::Error> {
        let affected_spectators = {
            let mut gm = self.ctx.game_manager.write().await;
            gm.cleanup_empty_games()
        };
        
        helpers::notify_spectators_of_game_deletion(
            &affected_spectators,
            self.ctx.game_manager.clone(),
            self.ctx.state_manager.clone(),
        ).await?;
        
        let gm = self.ctx.game_manager.read().await;
        let games_list = gm.get_all_games();
        drop(gm);
        
        let player_ids = game_helpers::collect_player_ids_from_games(&games_list);
        let player_names = self.ctx.state_manager.read().await.get_player_names_for_ids(&player_ids).await;
        
        let games_with_names = game_helpers::add_names_to_games(games_list, &player_names);
        
        server::send_json(&self.ctx.sender, &ServerMessage::GamesList {
            games: games_with_names,
        })
    }

    pub async fn join_game(&self) -> Result<(), axum::Error> {
        use crate::game_manager::QueueResult;
        
        let mut gm = self.ctx.game_manager.write().await;
        let result = gm.join_game_queue(self.ctx.player_id.clone());
        drop(gm);
        
        match result {
            QueueResult::Matched { game_id, room_id, white_player, black_player, color, game_started } => {
                server::send_json(&self.ctx.sender, &ServerMessage::Matched {
                    game_id: game_id.clone(),
                    room_id,
                    white_player,
                    black_player,
                    color: color.clone(),
                    game_started,
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
            QueueResult::InQueue { players_in_queue, position } => {
                server::send_json(&self.ctx.sender, &ServerMessage::QueueStatus {
                    status: QueueResult::InQueue { players_in_queue, position },
                })
            }
        }
    }

    pub async fn get_lobby_status(&self) -> Result<(), axum::Error> {
        let mut gm = self.ctx.game_manager.write().await;
        let status = gm.get_lobby_status(&self.ctx.player_id);
        drop(gm);
        
        let status_clone = status.clone();
        server::send_json(&self.ctx.sender, &ServerMessage::LobbyStatus { status: status_clone.clone() })?;
        
        if let LobbyStatus::InGame { game_id, .. } = &status {
            helpers::send_game_state_to_player(
                &self.ctx.player_id,
                game_id,
                self.ctx.game_manager.clone(),
                self.ctx.state_manager.clone(),
                self.ctx.sender.clone(),
            ).await?;
        }
        
        Ok(())
    }
}

