use super::ChessGame;
use super::types::GameStatus;
use shakmaty::{Color, Position};

impl ChessGame {
    pub fn is_game_over(&self) -> bool {
        if self.resigned_by.is_some() || self.timeout_by.is_some() || self.accept_draw_called || self.claimed_draw_by.is_some() {
            return true;
        }
        
        // Check for checkmate first (takes precedence over 75-move rule)
        if self.position.is_checkmate() {
            return true;
        }
        
        // FIDE 75-move rule: automatic draw after 75 moves (150 half-moves) without pawn move or capture
        // Exception: checkmate on 75th move takes precedence (already checked above)
        if self.position.halfmoves() >= 150 {
            return true;
        }
        
        self.position.is_stalemate() || self.position.is_insufficient_material()
    }

    pub fn get_result(&self) -> Option<String> {
        if let Some(timeout_result) = self.get_result_from_timeout() {
            return Some(timeout_result);
        }
        if let Some(resignation_result) = self.get_result_from_resignation() {
            return Some(resignation_result);
        }
        if self.accept_draw_called || self.claimed_draw_by.is_some() {
            return Some("1/2-1/2".to_string());
        }
        // Check board result - checkmate takes precedence over 75-move rule
        if self.position.is_checkmate() {
            return Some(if self.position.turn() == Color::White {
                "0-1".to_string()  // Black wins (it's White's turn but they're checkmated)
            } else {
                "1-0".to_string()  // White wins (it's Black's turn but they're checkmated)
            });
        }
        // FIDE 75-move rule: automatic draw after 75 moves (150 half-moves) without pawn move or capture
        if self.position.halfmoves() >= 150 {
            return Some("1/2-1/2".to_string());
        }
        if self.position.is_stalemate() || self.position.is_insufficient_material() {
            return Some("1/2-1/2".to_string());
        }
        None
    }

    pub fn is_check(&self) -> bool {
        self.position.is_check()
    }

    pub fn is_checkmate(&self) -> bool {
        self.position.is_checkmate()
    }
    
    pub fn is_stalemate(&self) -> bool {
        self.position.is_stalemate()
    }
    
    pub fn is_insufficient_material(&self) -> bool {
        self.position.is_insufficient_material()
    }

    pub fn offer_draw(&mut self, player_id: &str) -> bool {
        if !self.game_started {
            return false;
        }
        if !self.is_player_in_game(player_id) {
            return false;
        }
        if self.resigned_by.is_some() || self.accept_draw_called || self.claimed_draw_by.is_some() {
            return false;
        }

        if Some(player_id) == self.draw_offered_by.as_deref() {
            self.draw_offered_by = None;
            return true;
        }

        self.draw_offered_by = Some(player_id.to_string());
        true
    }

    pub fn accept_draw(&mut self, player_id: &str) -> bool {
        if !self.game_started {
            return false;
        }
        if self.draw_offered_by.is_none() {
            return false;
        }
        if Some(player_id) == self.draw_offered_by.as_deref() {
            return false;
        }
        if !self.is_player_in_game(player_id) {
            return false;
        }
        if self.resigned_by.is_some() {
            return false;
        }

        self.accept_draw_called = true;
        true
    }

    pub fn resign(&mut self, player_id: &str) -> bool {
        if !self.game_started {
            return false;
        }
        if !self.is_player_in_game(player_id) {
            return false;
        }
        if self.resigned_by.is_some() {
            return false;
        }

        self.resigned_by = Some(player_id.to_string());
        true
    }

    pub fn get_result_from_resignation(&self) -> Option<String> {
        if let Some(resigned_by) = &self.resigned_by {
            if Some(resigned_by.as_str()) == self.white_player.as_deref() {
                return Some("0-1".to_string());
            } else if Some(resigned_by.as_str()) == self.black_player.as_deref() {
                return Some("1-0".to_string());
            }
        }
        None
    }

    pub fn get_result_from_timeout(&self) -> Option<String> {
        if let Some(timeout_by) = &self.timeout_by {
            if Some(timeout_by.as_str()) == self.white_player.as_deref() {
                return Some("0-1".to_string());
            } else if Some(timeout_by.as_str()) == self.black_player.as_deref() {
                return Some("1-0".to_string());
            }
        }
        None
    }

    pub fn get_position_signature(&self) -> String {
        // Get FEN without move counters (last two numbers)
        // FEN format: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        // We need everything except the last two numbers
        use shakmaty::fen::Fen;
        let fen = Fen::from_position(&self.position, shakmaty::EnPassantMode::Legal);
        let fen_str = format!("{}", fen);
        // Split by spaces and take first 4 parts (board, turn, castling, en passant)
        // This excludes halfmove clock and fullmove number
        fen_str.split_whitespace()
            .take(4)
            .collect::<Vec<&str>>()
            .join(" ")
    }
    
    pub fn can_claim_threefold_repetition(&self) -> bool {
        if !self.game_started || self.position_signatures.is_empty() {
            return false;
        }
        
        // Get current position signature
        let current_signature = self.get_position_signature();
        
        // Count how many times this position has occurred
        // position_signatures already includes the current position (added after each move)
        let mut count = 0;
        for sig in &self.position_signatures {
            if sig == &current_signature {
                count += 1;
            }
        }
        
        // Threefold repetition: same position occurs 3 times
        count >= 3
    }
    
    pub fn can_claim_fifty_moves(&self) -> bool {
        self.position.halfmoves() >= 100
    }
    
    pub fn get_game_status(&self) -> GameStatus {
        GameStatus {
            is_game_over: self.is_game_over(),
            is_check: self.position.is_check(),
            is_checkmate: self.position.is_checkmate(),
            is_stalemate: self.position.is_stalemate(),
            is_insufficient_material: self.position.is_insufficient_material(),
            turn: self.get_turn(),
            result: self.get_result(),
            result_reason: self.get_result_reason(),
        }
    }
    
    pub fn get_result_reason(&self) -> Option<String> {
        if let Some(_) = self.timeout_by {
            return Some("timeout".to_string());
        }
        if let Some(_) = self.claimed_draw_by {
            return self.claimed_draw_reason.clone();
        }
        
        // Check for checkmate first (takes precedence over 75-move rule)
        if self.position.is_checkmate() {
            return Some("checkmate".to_string());
        }
        
        // FIDE 75-move rule: automatic draw after 75 moves (150 half-moves) without pawn move or capture
        if self.position.halfmoves() >= 150 {
            return Some("seventy_five_moves".to_string());
        }
        
        if self.position.is_stalemate() {
            Some("stalemate".to_string())
        } else if self.position.is_insufficient_material() {
            Some("insufficient_material".to_string())
        } else if self.accept_draw_called {
            Some("draw_by_agreement".to_string())
        } else {
            None
        }
    }
    
    pub fn get_result_message(&self) -> String {
        if !self.is_game_over() {
            if self.is_check() {
                return format!("{} is in check", self.get_turn());
            }
            return "Game in progress".to_string();
        }
        
        if let Some(result) = self.get_result() {
            let reason = self.get_result_reason().unwrap_or_default();
            
            match result.as_str() {
                "1-0" => {
                    if reason == "checkmate" {
                        "White wins by checkmate".to_string()
                    } else {
                        "White wins".to_string()
                    }
                }
                "0-1" => {
                    if reason == "checkmate" {
                        "Black wins by checkmate".to_string()
                    } else {
                        "Black wins".to_string()
                    }
                }
                "1/2-1/2" => {
                    match reason.as_str() {
                        "stalemate" => "Draw by stalemate".to_string(),
                        "insufficient_material" => "Draw by insufficient material".to_string(),
                        "draw_by_agreement" => "Draw by agreement".to_string(),
                        "threefold_repetition" => "Draw by threefold repetition (claimed)".to_string(),
                        "fifty_moves" => "Draw by fifty-move rule (claimed)".to_string(),
                        "seventy_five_moves" => "Draw by seventy-five-move rule".to_string(),
                        _ => "Draw".to_string(),
                    }
                }
                _ => "Game over".to_string(),
            }
        } else {
            "Game over".to_string()
        }
    }
    
    pub fn claim_threefold_repetition(&mut self, player_id: &str) -> bool {
        if !self.game_started {
            return false;
        }
        if !self.is_player_in_game(player_id) {
            return false;
        }
        if self.resigned_by.is_some() || self.accept_draw_called || self.claimed_draw_by.is_some() {
            return false;
        }
        
        let player_color = self.get_player_color(player_id);
        if player_color.as_deref() != Some(self.get_turn().as_str()) {
            return false;
        }
        
        if !self.can_claim_threefold_repetition() {
            return false;
        }
        
        self.claimed_draw_by = Some(player_id.to_string());
        self.claimed_draw_reason = Some("threefold_repetition".to_string());
        true
    }
    
    pub fn claim_fifty_moves(&mut self, player_id: &str) -> bool {
        if !self.game_started {
            return false;
        }
        if !self.is_player_in_game(player_id) {
            return false;
        }
        if self.resigned_by.is_some() || self.accept_draw_called || self.claimed_draw_by.is_some() {
            return false;
        }
        
        let player_color = self.get_player_color(player_id);
        if player_color.as_deref() != Some(self.get_turn().as_str()) {
            return false;
        }
        
        if !self.can_claim_fifty_moves() {
            return false;
        }
        
        self.claimed_draw_by = Some(player_id.to_string());
        self.claimed_draw_reason = Some("fifty_moves".to_string());
        true
    }
}

