use crate::game_manager::GameManager;
use crate::messages::server::{self, ServerMessage};
use crate::models::{StateManager, WebSocketSender};
use crate::utils::game_helpers;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn notify_spectators_of_game_deletion(
    spectator_ids: &[String],
    game_manager: Arc<RwLock<GameManager>>,
    state_manager: Arc<RwLock<StateManager>>,
) -> Result<(), axum::Error> {
    if spectator_ids.is_empty() {
        return Ok(());
    }
    
    let games_list = {
        let gm = game_manager.read().await;
        gm.get_all_games()
    };
    
    let player_ids = game_helpers::collect_player_ids_from_games(&games_list);
    let player_names = state_manager.read().await.get_player_names_for_ids(&player_ids).await;
    
    let games_with_names = game_helpers::add_names_to_games(games_list, &player_names);
    let games_msg = ServerMessage::GamesList { games: games_with_names };
    let games_json = serde_json::to_string(&games_msg).unwrap();
    
    let sm = state_manager.read().await;
    for spectator_id in spectator_ids {
        if let Some(spectator_sender) = sm.get_connection(spectator_id).await {
            let kicked_msg = ServerMessage::KickedToLobby {
                message: "Game room closed. Returning to lobby...".to_string(),
            };
            let json = serde_json::to_string(&kicked_msg).unwrap();
            let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
            
            let lobby_status = {
                let mut gm = game_manager.write().await;
                gm.get_lobby_status(spectator_id).clone()
            };
            let lobby_msg = ServerMessage::LobbyStatus { status: lobby_status };
            let json = serde_json::to_string(&lobby_msg).unwrap();
            let _ = spectator_sender.send(axum::extract::ws::Message::Text(json));
            
            let _ = spectator_sender.send(axum::extract::ws::Message::Text(games_json.clone()));
        }
    }
    
    Ok(())
}

pub async fn send_message_to_all_in_game(
    game_id: &str,
    message: ServerMessage,
    game_manager: Arc<RwLock<GameManager>>,
    state_manager: Arc<RwLock<StateManager>>,
) {
    let (player_ids, spectator_ids) = {
        let gm = game_manager.read().await;
        let game = match gm.get_game(game_id) {
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
        
        let spectator_ids = gm.get_spectators(game_id);
        (player_ids, spectator_ids)
    };
    
    let json = match serde_json::to_string(&message) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("Failed to serialize message: {}", e);
            return;
        }
    };
    
    let sm = state_manager.read().await;
    
    for player_id in &player_ids {
        if let Some(sender) = sm.get_connection(player_id).await {
            let _ = sender.send(axum::extract::ws::Message::Text(json.clone()));
        }
    }
    
    for spectator_id in &spectator_ids {
        if let Some(sender) = sm.get_connection(spectator_id).await {
            let _ = sender.send(axum::extract::ws::Message::Text(json.clone()));
        }
    }
}

pub async fn send_game_state_to_player(
    player_id: &str,
    game_id: &str,
    game_manager: Arc<RwLock<GameManager>>,
    state_manager: Arc<RwLock<StateManager>>,
    sender: WebSocketSender,
) -> Result<(), axum::Error> {
    let (player_color, waiting_for_opponent, board_dict, current_turn, game_status, room_id, game_started, white_ready, black_ready, draw_offered_by, resigned_by, accept_draw_called, can_claim_threefold, can_claim_fifty, can_rematch, rematch_requested_by, rematch_accepted_by, result, result_reason, result_message, fen, opponent_name, move_history, move_san_history, move_fens, white_player_name, black_player_name, white_player_id, black_player_id, claimed_draw_by, claimed_draw_reason, spectator_count, is_spectator, time_control, white_time_remaining, black_time_remaining) = {
        let gm = game_manager.read().await;
        let game = match gm.get_game(game_id) {
            Some(g) => g,
            None => return Ok(()),
        };
        
        let is_spectator = gm.get_spectators(game_id).contains(&player_id.to_string());
        let spectator_count = gm.get_spectator_count(game_id);
        
        let player_color = if is_spectator {
            None
        } else {
            game.get_player_color(player_id)
        };
        
        let waiting_for_opponent = (game.player1.is_some() && game.player2.is_none()) || 
                                  (game.player1.is_none() && game.player2.is_some());
        
        let board_dict_raw = if !game.game_started {
            HashMap::new()
        } else {
            game.get_board_dict(None)
        };
        
        use crate::chess_game::PieceInfo;
        let mut board_dict: HashMap<String, PieceInfo> = HashMap::new();
        for (square, piece_opt) in board_dict_raw {
            if let Some(piece) = piece_opt {
                board_dict.insert(square, piece);
            }
        }
        
        let current_turn = if game.colors_assigned { Some(game.get_turn()) } else { None };
        let game_status = game.get_game_status();
        
        let opponent_id = if is_spectator {
            None
        } else if game.colors_assigned {
            if Some(player_id) == game.white_player.as_deref() {
                game.black_player.clone()
            } else {
                game.white_player.clone()
            }
        } else {
            if Some(player_id) == game.player1.as_deref() {
                game.player2.clone()
            } else {
                game.player1.clone()
            }
        };
        
        let sm = state_manager.read().await;
        let opponent_name = if is_spectator {
            None
        } else if let Some(id) = &opponent_id {
            sm.get_player_name(id).await
        } else {
            None
        };
        let white_player_name = if let Some(id) = &game.white_player {
            sm.get_player_name(id).await
        } else {
            None
        };
        let black_player_name = if let Some(id) = &game.black_player {
            sm.get_player_name(id).await
        } else {
            None
        };
        drop(sm);
        
        let move_history = if game.game_started {
            game.get_move_history().iter().map(|e| e.uci.clone()).collect()
        } else {
            Vec::new()
        };
        let move_san_history = if game.game_started {
            game.get_move_history().iter().map(|e| e.san.clone()).collect()
        } else {
            Vec::new()
        };
        let move_fens = if game.game_started {
            game.get_move_fens()
        } else {
            vec![game.get_fen()]
        };
        
        let timeout_result = game.get_result_from_timeout();
        let resignation_result = game.get_result_from_resignation();
        let result_message = if let Some(ref res) = timeout_result {
            if is_spectator {
                let winner_name = if res == "1-0" {
                    white_player_name.clone().unwrap_or_else(|| "White".to_string())
                } else {
                    black_player_name.clone().unwrap_or_else(|| "Black".to_string())
                };
                Some(format!("{} won on time", winner_name))
            } else {
                let winner_text = if res == "1-0" {
                    if player_color.as_deref() == Some("white") { "You" } else { "Opponent" }
                } else {
                    if player_color.as_deref() == Some("black") { "You" } else { "Opponent" }
                };
                Some(format!("{} won on time", winner_text))
            }
        } else if let Some(ref res) = resignation_result {
            if is_spectator {
                let winner_name = if res == "1-0" {
                    white_player_name.clone().unwrap_or_else(|| "White".to_string())
                } else {
                    black_player_name.clone().unwrap_or_else(|| "Black".to_string())
                };
                Some(format!("{} won by resignation", winner_name))
            } else {
                let winner_text = if res == "1-0" {
                    if player_color.as_deref() == Some("white") { "You" } else { "Opponent" }
                } else {
                    if player_color.as_deref() == Some("black") { "You" } else { "Opponent" }
                };
                Some(format!("{} won by resignation", winner_text))
            }
        } else if let Some(ref _claimed_by) = game.claimed_draw_by {
            let reason = game.claimed_draw_reason.as_deref().unwrap_or("");
            if reason == "threefold_repetition" {
                Some("Draw by threefold repetition (claimed)".to_string())
            } else if reason == "fifty_moves" {
                Some("Draw by fifty-move rule (claimed)".to_string())
            } else {
                Some("Draw (claimed)".to_string())
            }
        } else if game.accept_draw_called {
            Some("Draw by agreement".to_string())
        } else if game_status.is_game_over {
            if let Some(result) = game.get_result() {
                let reason = game.get_result_reason().unwrap_or_default();
                if is_spectator {
                    match result.as_str() {
                        "1-0" => {
                            let winner_name = white_player_name.clone().unwrap_or_else(|| "White".to_string());
                            if !reason.is_empty() && reason != "checkmate" {
                                Some(format!("{} wins ({})", winner_name, reason))
                            } else if reason == "checkmate" {
                                Some(format!("{} wins by checkmate", winner_name))
                            } else {
                                Some(format!("{} wins", winner_name))
                            }
                        }
                        "0-1" => {
                            let winner_name = black_player_name.clone().unwrap_or_else(|| "Black".to_string());
                            if !reason.is_empty() && reason != "checkmate" {
                                Some(format!("{} wins ({})", winner_name, reason))
                            } else if reason == "checkmate" {
                                Some(format!("{} wins by checkmate", winner_name))
                            } else {
                                Some(format!("{} wins", winner_name))
                            }
                        }
                        "1/2-1/2" => {
                            match reason.as_str() {
                                "stalemate" => Some("Draw by stalemate".to_string()),
                                "insufficient_material" => Some("Draw by insufficient material".to_string()),
                                "draw_by_agreement" => Some("Draw by agreement".to_string()),
                                "threefold_repetition" => Some("Draw by threefold repetition (claimed)".to_string()),
                                "fifty_moves" => Some("Draw by fifty-move rule (claimed)".to_string()),
                                _ => {
                                    if !reason.is_empty() {
                                        Some(format!("Draw ({})", reason))
                                    } else {
                                        Some("Draw".to_string())
                                    }
                                }
                            }
                        }
                        _ => {
                            if !reason.is_empty() {
                                Some(format!("Game over ({})", reason))
                            } else {
                                Some("Game over".to_string())
                            }
                        }
                    }
                } else {
                    match result.as_str() {
                        "1-0" => {
                            if player_color.as_deref() == Some("white") {
                                if !reason.is_empty() && reason != "checkmate" {
                                    Some(format!("You win ({})", reason))
                                } else if reason == "checkmate" {
                                    Some("You win by checkmate".to_string())
                                } else {
                                    Some("You win".to_string())
                                }
                            } else {
                                if !reason.is_empty() && reason != "checkmate" {
                                    Some(format!("Opponent wins ({})", reason))
                                } else if reason == "checkmate" {
                                    Some("Opponent wins by checkmate".to_string())
                                } else {
                                    Some("Opponent wins".to_string())
                                }
                            }
                        }
                        "0-1" => {
                            if player_color.as_deref() == Some("black") {
                                if !reason.is_empty() && reason != "checkmate" {
                                    Some(format!("You win ({})", reason))
                                } else if reason == "checkmate" {
                                    Some("You win by checkmate".to_string())
                                } else {
                                    Some("You win".to_string())
                                }
                            } else {
                                if !reason.is_empty() && reason != "checkmate" {
                                    Some(format!("Opponent wins ({})", reason))
                                } else if reason == "checkmate" {
                                    Some("Opponent wins by checkmate".to_string())
                                } else {
                                    Some("Opponent wins".to_string())
                                }
                            }
                        }
                        "1/2-1/2" => {
                            match reason.as_str() {
                                "stalemate" => Some("Draw by stalemate".to_string()),
                                "insufficient_material" => Some("Draw by insufficient material".to_string()),
                                "draw_by_agreement" => Some("Draw by agreement".to_string()),
                                "threefold_repetition" => Some("Draw by threefold repetition (claimed)".to_string()),
                                "fifty_moves" => Some("Draw by fifty-move rule (claimed)".to_string()),
                                _ => {
                                    if !reason.is_empty() {
                                        Some(format!("Draw ({})", reason))
                                    } else {
                                        Some("Draw".to_string())
                                    }
                                }
                            }
                        }
                        _ => {
                            if !reason.is_empty() {
                                Some(format!("Game over ({})", reason))
                            } else {
                                Some("Game over".to_string())
                            }
                        }
                    }
                }
            } else {
                let reason = game.get_result_reason().unwrap_or_default();
                if !reason.is_empty() {
                    Some(format!("Game over ({})", reason))
                } else {
                    Some("Game over".to_string())
                }
            }
        } else {
            None
        };
        
        // Get time control and current time remaining
        let time_control_str = match game.time_control {
            crate::chess_game::TimeControl::Bullet => Some("bullet".to_string()),
            crate::chess_game::TimeControl::Blitz => Some("blitz".to_string()),
            crate::chess_game::TimeControl::Rapid => Some("rapid".to_string()),
            crate::chess_game::TimeControl::Classical => Some("classical".to_string()),
        };
        let (white_time, black_time) = game.get_current_time_remaining();
        
        (
            player_color,
            waiting_for_opponent,
            board_dict,
            current_turn,
            game_status,
            game.room_id.clone(),
            game.game_started,
            game.white_ready,
            game.black_ready,
            game.draw_offered_by.clone(),
            game.resigned_by.clone(),
            game.accept_draw_called,
            game.can_claim_threefold_repetition(),
            game.can_claim_fifty_moves(),
            game.is_game_over() || game.resigned_by.is_some(),
            game.rematch_requested_by.clone(),
            game.rematch_accepted_by.clone(),
            resignation_result.or(game.get_result()),
            game.get_result_reason(),
            result_message,
            game.get_fen(),
            opponent_name,
            move_history,
            move_san_history,
            move_fens,
            white_player_name,
            black_player_name,
            game.white_player.clone(),
            game.black_player.clone(),
            game.claimed_draw_by.clone(),
            game.claimed_draw_reason.clone(),
            spectator_count,
            is_spectator,
            time_control_str,
            Some(white_time),
            Some(black_time),
        )
    };
    
    let game_state = ServerMessage::GameState {
        game_id: game_id.to_string(),
        room_id: room_id.clone(),
        board: board_dict,
        turn: current_turn,
        your_color: player_color,
        is_game_over: can_rematch,
        is_check: game_status.is_check,
        is_checkmate: game_status.is_checkmate,
        is_stalemate: game_status.is_stalemate,
        result,
        result_reason,
        result_message,
        fen,
        waiting_for_opponent,
        game_started,
        white_ready,
        black_ready,
        draw_offered_by,
        resigned_by,
        accept_draw_called,
        can_claim_threefold_repetition: can_claim_threefold,
        can_claim_fifty_moves: can_claim_fifty,
        can_claim_draw: can_claim_threefold || can_claim_fifty,
        claimed_draw_by,
        claimed_draw_reason,
        can_rematch,
        rematch_requested_by,
        rematch_accepted_by,
        opponent_name,
        white_player_name,
        black_player_name,
        white_player: white_player_id,
        black_player: black_player_id,
        move_history,
        move_san_history,
        move_fens,
        spectator_count,
        is_spectator,
        time_control,
        white_time_remaining,
        black_time_remaining,
    };
    
    server::send_json(&sender, &game_state)
}

pub async fn send_game_state_to_all(
    game_id: &str,
    game_manager: Arc<RwLock<GameManager>>,
    state_manager: Arc<RwLock<StateManager>>,
) -> Result<(), axum::Error> {
    let (player_ids, spectator_ids) = {
        let gm = game_manager.read().await;
        let game = match gm.get_game(game_id) {
            Some(g) => g,
            None => return Ok(()),
        };
        
        let mut player_ids = Vec::new();
        if let Some(p1) = &game.player1 {
            player_ids.push(p1.clone());
        }
        if let Some(p2) = &game.player2 {
            player_ids.push(p2.clone());
        }
        
        let spectator_ids = gm.get_spectators(game_id);
        (player_ids, spectator_ids)
    };
    
    for player_id in &player_ids {
        if let Some(sender) = state_manager.read().await.get_connection(player_id).await {
            if let Err(e) = send_game_state_to_player(
                player_id,
                game_id,
                game_manager.clone(),
                state_manager.clone(),
                sender,
            ).await {
                tracing::warn!("Failed to send game state to player {}: {}", &player_id[..8], e);
            }
        }
    }
    
    for spectator_id in &spectator_ids {
        if let Some(sender) = state_manager.read().await.get_connection(spectator_id).await {
            if let Err(e) = send_game_state_to_player(
                spectator_id,
                game_id,
                game_manager.clone(),
                state_manager.clone(),
                sender,
            ).await {
                tracing::warn!("Failed to send game state to spectator {}: {}", &spectator_id[..8], e);
            }
        }
    }
    
    Ok(())
}

