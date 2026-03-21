use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "create_room")]
    CreateRoom {
        password: Option<String>,
        allow_spectators: Option<bool>,
        time_control: Option<String>,
    },
    #[serde(rename = "join_room")]
    JoinRoom {
        room_id: String,
        password: Option<String>,
    },
    #[serde(rename = "get_games_list")]
    GetGamesList,
    #[serde(rename = "join_game")]
    JoinGame,
    #[serde(rename = "get_lobby_status")]
    GetLobbyStatus,
    #[serde(rename = "make_move")]
    MakeMove {
        #[serde(rename = "move")]
        move_uci: String,
    },
    #[serde(rename = "get_legal_moves")]
    GetLegalMoves {
        square: String,
    },
    #[serde(rename = "player_ready")]
    PlayerReady,
    #[serde(rename = "offer_draw")]
    OfferDraw,
    #[serde(rename = "accept_draw")]
    AcceptDraw,
    #[serde(rename = "resign")]
    Resign,
    #[serde(rename = "claim_threefold_repetition")]
    ClaimThreefoldRepetition,
    #[serde(rename = "claim_fifty_moves")]
    ClaimFiftyMoves,
    #[serde(rename = "request_rematch")]
    RequestRematch,
    #[serde(rename = "accept_rematch")]
    AcceptRematch,
    #[serde(rename = "reject_rematch")]
    RejectRematch,
    #[serde(rename = "get_game_state")]
    GetGameState,
    #[serde(rename = "set_player_name")]
    SetPlayerName {
        name: String,
    },
    #[serde(rename = "leave_game")]
    LeaveGame,
    #[serde(rename = "spectate_room")]
    SpectateRoom {
        room_id: String,
        password: Option<String>,
    },
}

