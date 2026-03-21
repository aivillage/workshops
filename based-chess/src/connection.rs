use axum::extract::ws::WebSocket;
use crate::game_manager::GameManager;
use crate::messages;
use crate::models::StateManager;
use crate::utils::game_helpers;
use crate::websocket::WebSocketHandler;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub game_manager: Arc<RwLock<GameManager>>,
    pub state_manager: Arc<RwLock<StateManager>>,
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    let player_id = Uuid::new_v4().to_string();
    let default_name = format!("Anonymous {}", &player_id[..8].to_uppercase());
    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<axum::extract::ws::Message>();
    
    {
        let sm = state.state_manager.write().await;
        sm.set_connection(player_id.clone(), tx.clone()).await;
        sm.set_player_name(player_id.clone(), default_name.clone()).await;
    }
    
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });
    
    let connected_msg = messages::ServerMessage::Connected {
        player_id: player_id.clone(),
        player_name: default_name.clone(),
    };
    let json = serde_json::to_string(&connected_msg).unwrap();
    if tx.send(axum::extract::ws::Message::Text(json)).is_err() {
        send_task.abort();
        return;
    }
    
    let lobby_status = {
        let mut gm = state.game_manager.write().await;
        gm.join_lobby(player_id.clone())
    };
    
    let lobby_msg = messages::ServerMessage::LobbyStatus { status: lobby_status };
    let json = serde_json::to_string(&lobby_msg).unwrap();
    if tx.send(axum::extract::ws::Message::Text(json)).is_err() {
        send_task.abort();
        return;
    }
    
    let games_list = {
        let gm = state.game_manager.read().await;
        gm.get_all_games()
    };
    
    let player_ids = game_helpers::collect_player_ids_from_games(&games_list);
    let player_names = state.state_manager.read().await.get_player_names_for_ids(&player_ids).await;
    
    let games_with_names = game_helpers::add_names_to_games(games_list, &player_names);
    let games_msg = messages::ServerMessage::GamesList { games: games_with_names };
    let json = serde_json::to_string(&games_msg).unwrap();
    if tx.send(axum::extract::ws::Message::Text(json)).is_err() {
        send_task.abort();
        return;
    }
    
    let mut handler = WebSocketHandler::new(
        player_id.clone(),
        state.game_manager.clone(),
        state.state_manager.clone(),
        tx.clone(),
    );
    
    let mut rx_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    match serde_json::from_str::<messages::ClientMessage>(&text) {
                        Ok(client_msg) => {
                            tracing::debug!("Received message: {:?}", client_msg);
                            if handler.handle_message(client_msg).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse message: {} - Raw message: {}", e, text);
                            let error_msg = messages::ServerMessage::Error {
                                message: format!("Invalid message: {}", e),
                            };
                            let json = serde_json::to_string(&error_msg).unwrap();
                            if tx.send(axum::extract::ws::Message::Text(json)).is_err() {
                                break;
                            }
                        }
                    }
                }
                axum::extract::ws::Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });
    
    tokio::select! {
        _ = &mut send_task => {
            rx_task.abort();
        }
        _ = &mut rx_task => {
            send_task.abort();
        }
    }
    
    cleanup_disconnect(&player_id, &state).await;
    
    tracing::info!("Player {} disconnected", &player_id[..8]);
}

async fn cleanup_disconnect(player_id: &str, state: &AppState) {
    let (
        game_id,
        was_host,
        was_spectator,
        host_id,
        players_to_notify,
        spectators_to_notify,
    ): (
        Option<String>,
        bool,
        bool,
        Option<String>,
        Vec<String>,
        Vec<String>,
    ) = {
        let gm = state.game_manager.read().await;
        if let Some(game) = gm.get_player_game(player_id) {
            let was_host = game.player1.as_deref() == Some(player_id);
            let host_id = game.player1.clone();
            let game_id = game.game_id.clone();
            
            let mut players_to_notify = Vec::new();
            
            if was_host {
                if game.colors_assigned {
                    if let Some(ref wp) = game.white_player {
                        if wp != player_id {
                            players_to_notify.push(wp.clone());
                        }
                    }
                    if let Some(ref bp) = game.black_player {
                        if bp != player_id {
                            players_to_notify.push(bp.clone());
                        }
                    }
                } else {
                    if let Some(ref p1) = game.player1 {
                        if p1 != player_id {
                            players_to_notify.push(p1.clone());
                        }
                    }
                    if let Some(ref p2) = game.player2 {
                        if p2 != player_id {
                            players_to_notify.push(p2.clone());
                        }
                    }
                }
            }

            let spectators_to_notify = gm.get_spectators(&game_id);
            
            (
                Some(game_id),
                was_host,
                false,
                host_id,
                players_to_notify,
                spectators_to_notify,
            )
        } else if let Some(spectated_game_id) = gm.get_spectated_game_for_player(player_id) {
            (
                Some(spectated_game_id),
                false,
                true,
                None::<String>,
                Vec::<String>::new(),
                Vec::<String>::new(),
            )
        } else {
            (
                None,
                false,
                false,
                None::<String>,
                Vec::<String>::new(),
                Vec::<String>::new(),
            )
        }
    };
    
    {
        let mut gm = state.game_manager.write().await;
        if was_spectator {
            gm.remove_spectator(player_id);
        } else {
            gm.remove_player(player_id);
        }
    }
    
    if was_host {
        let all_affected: Vec<String> = {
            let mut affected = players_to_notify.clone();
            affected.extend(spectators_to_notify.clone());
            affected
        };
        
        let games_list = {
            let gm = state.game_manager.read().await;
            gm.get_all_games()
        };
        
        let player_ids = game_helpers::collect_player_ids_from_games(&games_list);
        let player_names = state.state_manager.read().await.get_player_names_for_ids(&player_ids).await;
        
        let games_with_names = game_helpers::add_names_to_games(games_list, &player_names);
        let games_msg = messages::ServerMessage::GamesList { games: games_with_names };
        let games_json = serde_json::to_string(&games_msg).unwrap();
        
        for other_player_id in players_to_notify {
            if let Some(player_sender) = state.state_manager.read().await.get_connection(&other_player_id).await {
                let kicked_msg = messages::ServerMessage::KickedToLobby {
                    message: "Host left the game. Returning to lobby...".to_string(),
                };
                let json = serde_json::to_string(&kicked_msg).unwrap();
                let _ = player_sender.send(axum::extract::ws::Message::Text(json));
                
                let lobby_status = {
                    let mut gm = state.game_manager.write().await;
                    gm.get_lobby_status(&other_player_id)
                };
                let lobby_msg = messages::ServerMessage::LobbyStatus { status: lobby_status };
                let json = serde_json::to_string(&lobby_msg).unwrap();
                let _ = player_sender.send(axum::extract::ws::Message::Text(json));
                
                let _ = player_sender.send(axum::extract::ws::Message::Text(games_json.clone()));
            }
        }
        
        for spectator_id in spectators_to_notify {
            if let Some(spectator_sender) = state.state_manager.read().await.get_connection(&spectator_id).await {
                let kicked_msg = messages::ServerMessage::KickedToLobby {
                    message: "Game room closed. Returning to lobby...".to_string(),
                };
                let json = serde_json::to_string(&kicked_msg).unwrap();
                let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
                
                let lobby_status = {
                    let mut gm = state.game_manager.write().await;
                    gm.get_lobby_status(&spectator_id)
                };
                let lobby_msg = messages::ServerMessage::LobbyStatus { status: lobby_status };
                let json = serde_json::to_string(&lobby_msg).unwrap();
                let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
                
                let _ = spectator_sender.send(axum::extract::ws::Message::Text(games_json.clone()));
            }
        }
        
        let exclude: HashSet<String> = {
            let gm = state.game_manager.read().await;
            let mut exclude = gm.get_busy_player_ids();
            exclude.insert(player_id.to_string());
            exclude.extend(all_affected);
            exclude
        };
        let lobby_connections = state.state_manager.read().await.get_connections_excluding(&exclude).await;
        for (_, sender) in lobby_connections {
            let _ = sender.send(axum::extract::ws::Message::Text(games_json.clone()));
        }
    } else if let Some(game_id) = game_id {
        let (game_exists, host_still_has_mapping) = {
            let gm = state.game_manager.read().await;
            let game_exists = gm.get_game(&game_id).is_some();
            let host_still_has_mapping = host_id.as_ref()
                .map(|h| gm.get_player_game(h.as_str()).is_some())
                .unwrap_or(false);
            (game_exists, host_still_has_mapping)
        };
        
        if game_exists && host_still_has_mapping {
            let (player_ids, spectator_ids) = {
                let gm = state.game_manager.read().await;
                let game = match gm.get_game(&game_id) {
                    Some(g) => g,
                    None => return,
                };
                
                let mut player_ids = Vec::new();
                if let Some(p1) = &game.player1 {
                    player_ids.push(p1.clone());
                }
                if let Some(p2) = &game.player2 {
                    player_ids.push(p2.clone());
                }
                if let Some(wp) = &game.white_player {
                    if !player_ids.contains(wp) {
                        player_ids.push(wp.clone());
                    }
                }
                if let Some(bp) = &game.black_player {
                    if !player_ids.contains(bp) {
                        player_ids.push(bp.clone());
                    }
                }
                
                let spectator_ids = gm.get_spectators(&game_id);
                (player_ids, spectator_ids)
            };
            
            let left_player_name = {
                let sm = state.state_manager.read().await;
                sm.get_player_name(player_id).await
            };
            
            let sm = state.state_manager.read().await;
            
            for player_id in &player_ids {
                if let Some(sender) = sm.get_connection(player_id).await {
                    let msg = messages::ServerMessage::OpponentLeft {
                        opponent_name: left_player_name.clone(),
                        message: if let Some(name) = &left_player_name {
                            Some(format!("{} left the game. Waiting for a new opponent to join...", name))
                        } else {
                            Some("Opponent left the game. Waiting for a new opponent to join...".to_string())
                        },
                    };
                    let json = serde_json::to_string(&msg).unwrap();
                    let _ = sender.send(axum::extract::ws::Message::Text(json));
                }
            }
            
            for spectator_id in &spectator_ids {
                if let Some(sender) = sm.get_connection(spectator_id).await {
                    let msg = messages::ServerMessage::OpponentLeft {
                        opponent_name: left_player_name.clone(),
                        message: if let Some(name) = &left_player_name {
                            Some(format!("{} left the game.", name))
                        } else {
                            Some("A player left the game.".to_string())
                        },
                    };
                    let json = serde_json::to_string(&msg).unwrap();
                    let _ = sender.send(axum::extract::ws::Message::Text(json));
                }
            }
            
            if let Some(ref host_id) = host_id {
                if let Some(host_sender) = sm.get_connection(host_id).await {
                    let mut temp_handler = WebSocketHandler::new(
                        host_id.clone(),
                        state.game_manager.clone(),
                        state.state_manager.clone(),
                        host_sender.clone(),
                    );
                    // Send game state via GetGameState message
                    let _ = temp_handler.handle_message(messages::ClientMessage::GetGameState).await;
                }
            }
        }
    }
    
    {
        let sm = state.state_manager.write().await;
        sm.remove_connection(player_id).await;
        sm.remove_player_name(player_id).await;
    }
}

