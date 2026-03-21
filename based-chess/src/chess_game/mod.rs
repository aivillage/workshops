use shakmaty::{Chess, Color, Move, Position};
use std::time::Instant;

pub mod types;
pub mod moves;
pub mod rules;

pub use types::*;

#[derive(Debug, Clone)]
pub struct ChessGame {
    pub game_id: String,
    pub room_id: String,
    pub password: Option<String>,
    pub allow_spectators: bool,
    pub position: Chess,
    pub white_player: Option<String>,
    pub black_player: Option<String>,
    pub player1: Option<String>,
    pub player2: Option<String>,
    pub colors_assigned: bool,
    pub white_ready: bool,
    pub black_ready: bool,
    pub game_started: bool,
    pub draw_offered_by: Option<String>,
    pub resigned_by: Option<String>,
    pub accept_draw_called: bool,
    pub claimed_draw_by: Option<String>,
    pub claimed_draw_reason: Option<String>,
    pub rematch_requested_by: Option<String>,
    pub rematch_accepted_by: Option<String>,
    pub move_history: Vec<Move>,
    pub position_signatures: Vec<String>, // Track position signatures for threefold repetition
    pub time_control: TimeControl,
    pub white_time_remaining: u64, // seconds
    pub black_time_remaining: u64, // seconds
    pub last_move_timestamp: Option<Instant>,
    pub timer_started: bool,
    pub timeout_by: Option<String>, // Player who timed out
}

impl ChessGame {
    pub fn new(game_id: String, room_id: String, password: Option<String>, allow_spectators: bool, time_control: TimeControl) -> Self {
        let initial_time = time_control.initial_time_seconds();
        Self {
            game_id,
            room_id,
            password,
            allow_spectators,
            position: Chess::new(),
            white_player: None,
            black_player: None,
            player1: None,
            player2: None,
            colors_assigned: false,
            white_ready: false,
            black_ready: false,
            game_started: false,
            draw_offered_by: None,
            resigned_by: None,
            accept_draw_called: false,
            claimed_draw_by: None,
            claimed_draw_reason: None,
            rematch_requested_by: None,
            rematch_accepted_by: None,
            move_history: Vec::new(),
            position_signatures: Vec::new(),
            time_control,
            white_time_remaining: initial_time,
            black_time_remaining: initial_time,
            last_move_timestamp: None,
            timer_started: false,
            timeout_by: None,
        }
    }

    pub fn assign_players(&mut self, white: String, black: Option<String>) {
        self.white_player = Some(white.clone());
        self.black_player = black.clone();
        self.player1 = Some(white);
        self.player2 = black;
        self.colors_assigned = true;
        self.white_ready = false;
        self.black_ready = false;
        self.game_started = false;
        self.draw_offered_by = None;
        self.resigned_by = None;
        self.accept_draw_called = false;
        self.claimed_draw_by = None;
        self.claimed_draw_reason = None;
        self.rematch_requested_by = None;
        self.rematch_accepted_by = None;
    }

    pub fn add_player(&mut self, player_id: String) -> Result<(), String> {
        if self.player1.is_none() {
            self.player1 = Some(player_id);
        } else if self.player2.is_none() {
            self.player2 = Some(player_id);
        } else {
            return Err("Room is full".to_string());
        }
        Ok(())
    }

    pub fn assign_colors_randomly(&mut self) -> bool {
        if self.player1.is_none() || self.player2.is_none() {
            return false;
        }
        if self.colors_assigned {
            return false;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.5) {
            self.white_player = self.player1.clone();
            self.black_player = self.player2.clone();
        } else {
            self.white_player = self.player2.clone();
            self.black_player = self.player1.clone();
        }

        self.colors_assigned = true;
        // Reset ready flags after assigning colors - they'll be set again when players click ready
        self.white_ready = false;
        self.black_ready = false;
        true
    }

    pub fn set_player_ready(&mut self, player_id: &str) -> bool {
        if !self.colors_assigned {
            // Colors not assigned yet - track readiness by player1/player2
            if Some(player_id) == self.player1.as_deref() {
                self.white_ready = true;  // Use white_ready to track player1 readiness
            } else if Some(player_id) == self.player2.as_deref() {
                self.black_ready = true;  // Use black_ready to track player2 readiness
            } else {
                return false;
            }

            // Check if both players are ready
            if self.player1.is_some() && self.player2.is_some() && self.white_ready && self.black_ready {
                // Both ready - assign colors randomly and start game
                if self.assign_colors_randomly() {
                    // After assigning colors, both players are considered ready for the game
                    // (they already clicked ready before colors were assigned)
                    self.white_ready = true;
                    self.black_ready = true;
                    self.game_started = true;
                    // Start timer when game starts
                    self.start_timer();
                    // Track starting position for threefold repetition detection
                    let starting_signature = self.get_position_signature();
                    self.position_signatures.push(starting_signature);
                    return true;
                }
                return false;
            }
            return false;
        } else {
            // Colors already assigned - use normal logic
            if Some(player_id) == self.white_player.as_deref() {
                self.white_ready = true;
            } else if Some(player_id) == self.black_player.as_deref() {
                self.black_ready = true;
            } else {
                return false;
            }

            // Check if both players are ready
            if self.white_player.is_some() && self.black_player.is_some() && self.white_ready && self.black_ready {
                self.game_started = true;
                // Start timer when game starts
                self.start_timer();
                // Track starting position for threefold repetition detection
                let starting_signature = self.get_position_signature();
                self.position_signatures.push(starting_signature);
                return true;
            }
            return false;
        }
    }

    pub fn get_fen(&self) -> String {
        use shakmaty::fen::Fen;
        let fen = Fen::from_position(&self.position, shakmaty::EnPassantMode::Legal);
        format!("{}", fen)
    }

    pub fn get_turn(&self) -> String {
        match self.position.turn() {
            Color::White => "white".to_string(),
            Color::Black => "black".to_string(),
        }
    }

    pub fn get_player_color(&self, player_id: &str) -> Option<String> {
        if !self.colors_assigned {
            return None;
        }
        if Some(player_id) == self.white_player.as_deref() {
            Some("white".to_string())
        } else if Some(player_id) == self.black_player.as_deref() {
            Some("black".to_string())
        } else {
            None
        }
    }

    pub fn is_player_in_game(&self, player_id: &str) -> bool {
        Some(player_id) == self.player1.as_deref() || Some(player_id) == self.player2.as_deref()
    }

    pub fn has_password(&self) -> bool {
        self.password.is_some() && !self.password.as_ref().unwrap().is_empty()
    }

    pub fn reset_game_for_rematch(&mut self) -> bool {
        self.position = Chess::new();
        self.move_history.clear();
        self.position_signatures.clear();
        self.resigned_by = None;
        self.accept_draw_called = false;
        self.claimed_draw_by = None;
        self.claimed_draw_reason = None;
        self.white_ready = false;
        self.black_ready = false;
        self.game_started = false;
        self.colors_assigned = false;
        self.white_player = None;
        self.black_player = None;
        self.rematch_requested_by = None;
        self.rematch_accepted_by = None;
        self.draw_offered_by = None;
        let initial_time = self.time_control.initial_time_seconds();
        self.white_time_remaining = initial_time;
        self.black_time_remaining = initial_time;
        self.last_move_timestamp = None;
        self.timer_started = false;
        self.timeout_by = None;
        true
    }

    pub fn start_timer(&mut self) {
        if !self.timer_started && self.game_started {
            self.timer_started = true;
            self.last_move_timestamp = Some(Instant::now());
        }
    }

    pub fn update_time_on_move(&mut self) -> (u64, u64, bool) {
        if !self.timer_started || !self.game_started {
            return (self.white_time_remaining, self.black_time_remaining, false);
        }

        let now = Instant::now();
        if let Some(last_timestamp) = self.last_move_timestamp {
            let elapsed = now.duration_since(last_timestamp).as_secs();
            
            // Determine which player just moved (the one whose turn it was before the move)
            // After a move, the turn switches, so the previous turn is the one who moved
            let previous_turn = if self.get_turn() == "white" {
                "black"
            } else {
                "white"
            };

            // Decrement time for the player who just moved
            if previous_turn == "white" {
                if elapsed >= self.white_time_remaining {
                    self.white_time_remaining = 0;
                    self.last_move_timestamp = Some(now);
                    return (0, self.black_time_remaining, true); // Timeout occurred
                } else {
                    self.white_time_remaining -= elapsed;
                    // Apply increment for white after a successful move
                    self.white_time_remaining += self.time_control.increment_seconds();
                }
            } else {
                if elapsed >= self.black_time_remaining {
                    self.black_time_remaining = 0;
                    self.last_move_timestamp = Some(now);
                    return (self.white_time_remaining, 0, true); // Timeout occurred
                } else {
                    self.black_time_remaining -= elapsed;
                    // Apply increment for black after a successful move
                    self.black_time_remaining += self.time_control.increment_seconds();
                }
            }
        }
        
        self.last_move_timestamp = Some(now);
        (self.white_time_remaining, self.black_time_remaining, false)
    }

    pub fn get_current_time_remaining(&self) -> (u64, u64) {
        if !self.timer_started || !self.game_started {
            return (self.white_time_remaining, self.black_time_remaining);
        }

        let now = Instant::now();
        if let Some(last_timestamp) = self.last_move_timestamp {
            let elapsed = now.duration_since(last_timestamp).as_secs();
            
            // Calculate current time for the player whose turn it is
            let current_turn = self.get_turn();
            let (mut white_time, mut black_time) = (self.white_time_remaining, self.black_time_remaining);
            
            if current_turn == "white" {
                if elapsed >= white_time {
                    white_time = 0;
                } else {
                    white_time -= elapsed;
                }
            } else {
                if elapsed >= black_time {
                    black_time = 0;
                } else {
                    black_time -= elapsed;
                }
            }
            
            return (white_time, black_time);
        }
        
        (self.white_time_remaining, self.black_time_remaining)
    }
}

