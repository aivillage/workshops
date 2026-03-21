use super::ChessGame;
use shakmaty::{Color, Position, Square, san::San};
use std::collections::HashMap;
use super::types::PieceInfo;

impl ChessGame {
    pub fn is_valid_move(&self, move_uci: &str) -> bool {
        if !self.game_started {
            return false;
        }
        
        // Normalize the input UCI string (lowercase)
        let move_uci_normalized = move_uci.to_lowercase();
        
        // Check if this move is in the list of legal moves by comparing UCI strings
        let legal_moves = self.position.legal_moves();
        legal_moves.iter().any(|m| {
            let legal_uci = m.to_uci(shakmaty::CastlingMode::Standard).to_string().to_lowercase();
            legal_uci == move_uci_normalized
        })
    }

    pub fn make_move(&mut self, move_uci: &str) -> bool {
        if !self.game_started {
            return false;
        }
        
        // Normalize the input UCI string (lowercase)
        let move_uci_normalized = move_uci.to_lowercase();
        
        // Find matching legal move by comparing UCI strings
        let legal_moves = self.position.legal_moves();
        if let Some(m) = legal_moves.iter().find(|m| {
            let legal_uci = m.to_uci(shakmaty::CastlingMode::Standard).to_string().to_lowercase();
            legal_uci == move_uci_normalized
        }) {
            let m_clone = m.clone();
            self.position.play_unchecked(m_clone.clone());
            // Track move in history
            self.move_history.push(m_clone);
            // Track position signature for threefold repetition detection
            let signature = self.get_position_signature();
            self.position_signatures.push(signature);
            // Clear draw offer when a move is made
            self.draw_offered_by = None;
            // Update time on move and check for timeout
            let (_, _, timeout_occurred) = self.update_time_on_move();
            if timeout_occurred {
                // Determine which player timed out (the one who just moved)
                let previous_turn = if self.get_turn() == "white" {
                    "black"
                } else {
                    "white"
                };
                if previous_turn == "white" {
                    if let Some(white_id) = &self.white_player {
                        self.timeout_by = Some(white_id.clone());
                    }
                } else {
                    if let Some(black_id) = &self.black_player {
                        self.timeout_by = Some(black_id.clone());
                    }
                }
            }
            true
        } else {
            false
        }
    }
    
    pub fn get_legal_moves(&self) -> Vec<String> {
        self.position.legal_moves()
            .iter()
            .map(|m| m.to_uci(shakmaty::CastlingMode::Standard).to_string())
            .collect()
    }
    
    pub fn get_legal_moves_for_square(&self, square_name: &str) -> Vec<String> {
        let square = match Square::from_ascii(square_name.as_bytes()) {
            Ok(sq) => sq,
            Err(_) => return Vec::new(),
        };
        
        self.position.legal_moves()
            .iter()
            .filter(|m| m.from() == Some(square))
            .map(|m| m.to_uci(shakmaty::CastlingMode::Standard).to_string())
            .collect()
    }

    pub fn get_move_history(&self) -> Vec<super::types::MoveHistoryEntry> {
        let mut history = Vec::new();
        let mut temp_position = shakmaty::Chess::new();
        let mut move_number = 1;
        
        for m in &self.move_history {
            let is_white = temp_position.turn() == Color::White;
            let uci = m.to_uci(shakmaty::CastlingMode::Standard).to_string();
            
            // Convert to SAN notation using shakmaty
            let mut san = San::from_move(&temp_position, m.clone()).to_string();
            
            // Play the move to check if it results in check or checkmate
            temp_position.play_unchecked(m.clone());
            
            // Add check or checkmate symbols
            if temp_position.is_checkmate() {
                san.push('#');
            } else if temp_position.is_check() {
                san.push('+');
            }
            
            history.push(super::types::MoveHistoryEntry {
                uci,
                san,
                move_number,
                is_white,
            });
            
            // Increment move number after black's move (completing the full move)
            if !is_white {
                move_number += 1;
            }
        }
        
        history
    }
    
    pub fn get_move_fens(&self) -> Vec<String> {
        let mut fens = Vec::new();
        let mut temp_position = shakmaty::Chess::new();
        
        use shakmaty::fen::Fen;
        fens.push(format!("{}", Fen::from_position(&temp_position, shakmaty::EnPassantMode::Legal)));
        
        for m in &self.move_history {
            temp_position.play_unchecked(m.clone());
            fens.push(format!("{}", Fen::from_position(&temp_position, shakmaty::EnPassantMode::Legal)));
        }
        
        fens
    }

    pub fn get_board_dict(&self, player_color: Option<&str>) -> HashMap<String, Option<PieceInfo>> {
        let mut board_dict = HashMap::new();
        let board = self.position.board();
        
        for square in Square::ALL {
            let piece = board.piece_at(square);
            let square_name = square.to_string();
            
            if let Some(p) = piece {
                let piece_color = match p.color {
                    Color::White => "white",
                    Color::Black => "black",
                };
                
                // If player_color is specified, only show their pieces
                if let Some(pc) = player_color {
                    if piece_color != pc {
                        board_dict.insert(square_name, None);
                        continue;
                    }
                }
                
                let piece_type = match p.role {
                    shakmaty::Role::King => if p.color == Color::White { "K" } else { "k" },
                    shakmaty::Role::Queen => if p.color == Color::White { "Q" } else { "q" },
                    shakmaty::Role::Rook => if p.color == Color::White { "R" } else { "r" },
                    shakmaty::Role::Bishop => if p.color == Color::White { "B" } else { "b" },
                    shakmaty::Role::Knight => if p.color == Color::White { "N" } else { "n" },
                    shakmaty::Role::Pawn => if p.color == Color::White { "P" } else { "p" },
                };
                
                board_dict.insert(square_name, Some(PieceInfo {
                    piece_type: piece_type.to_string(),
                    color: piece_color.to_string(),
                }));
            } else {
                board_dict.insert(square_name, None);
            }
        }
        
        board_dict
    }
}

