# based.mobi Chess Server

A real-time multiplayer chess server written in Rust, implementing FIDE-compliant chess rules with WebSocket-based communication. The server enforces all chess rules on the backend using the `shakmaty` library, maintains authoritative game clocks, and supports room-based matchmaking with spectator capabilities.

## Overview

This is a minimal, opinionated chess server that prioritizes:
- **FIDE rule compliance**: Threefold repetition, fifty-move rule (claimable), seventy-five-move rule (automatic), proper time controls
- **Server-side authority**: All move validation, game state, and timing logic runs on the server
- **In-memory state**: No database persistence; games exist only while players are connected
- **Privacy-first**: No user accounts, no analytics, no persistent game storage on the server

## Architecture

The server is built on **Axum** (async web framework) and **Tokio** (async runtime). All game logic runs in a single process with shared state protected by `Arc<RwLock<>>` for concurrent access.

### Core Components

- **WebSocket Handler** (`/ws`): Single endpoint for all client communication
- **Game Manager**: Manages active games, rooms, matchmaking queue, and spectators
- **Chess Game**: Wraps `shakmaty::Chess` with game state, timers, and draw/resign logic
- **State Manager**: Tracks WebSocket connections and player names
- **Message Protocol**: JSON-based request/response over WebSocket

### Concurrency Model

- **Shared State**: `GameManager` and `StateManager` wrapped in `Arc<RwLock<>>`
- **Background Tasks**: Timer task runs every 2 seconds to update game clocks
- **Per-Connection Tasks**: Each WebSocket connection spawns separate send/receive tasks
- **Lock Strategy**: Read locks for queries, write locks for mutations; locks held briefly

## Project Structure

```
src/
├── main.rs                 # Axum server setup, routing, background tasks
├── connection.rs           # WebSocket connection handling, cleanup on disconnect
├── websocket.rs            # WebSocket message routing to handlers
├── chess_game/             # Chess game logic
│   ├── mod.rs             # ChessGame struct, timer logic, player management
│   ├── moves.rs           # Move validation and execution
│   ├── rules.rs           # Game end conditions, draw claims, resignations
│   └── types.rs           # TimeControl, PieceInfo, GameStatus
├── game_manager/           # Game and room management
│   ├── mod.rs             # GameManager struct
│   ├── games.rs           # Game CRUD operations
│   ├── rooms.rs           # Room creation, joining, password protection
│   ├── lobby.rs           # Matchmaking queue logic
│   ├── spectators.rs       # Spectator management
│   └── types.rs           # LobbyStatus, QueueResult
├── handlers/               # Request handlers organized by domain
│   ├── base.rs            # HandlerContext trait
│   ├── game_handler.rs    # Make move, get game state, legal moves
│   ├── room_handler.rs    # Create room, join room, leave room
│   ├── draw_handler.rs    # Offer draw, accept draw, claim draw
│   ├── rematch_handler.rs # Request rematch, accept rematch
│   ├── lobby_handler.rs   # Join queue, get games list
│   ├── player_handler.rs  # Set player name
│   └── helpers.rs         # Shared utilities for sending game state
├── messages/               # WebSocket message types
│   ├── client.rs          # ClientMessage enum (incoming)
│   └── server.rs          # ServerMessage enum (outgoing)
├── models/                 # State management
│   └── mod.rs             # StateManager (connections, player names)
├── services/               # Business logic services
│   └── message_service.rs # Message broadcasting utilities
└── utils/                  # Utilities
    ├── cache_busting.rs   # HTML cache busting injection
    ├── game_helpers.rs    # Game info formatting
    └── piece_images.rs    # Piece image loading
```

## Key Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `axum` | Web framework with WebSocket support | 0.7 |
| `tokio` | Async runtime | 1.x |
| `shakmaty` | Chess rules engine (move validation, FEN, legal moves) | 0.28 |
| `serde` / `serde_json` | JSON serialization for WebSocket messages | 1.0 |
| `uuid` | Player ID generation | 1.6 |
| `tracing` | Structured logging | 0.1 |

### Why These Dependencies?

- **axum**: Modern, type-safe web framework with excellent WebSocket support and zero-cost abstractions
- **shakmaty**: Pure Rust chess library with FIDE-compliant rule enforcement; no external dependencies
- **tokio**: Industry-standard async runtime; required by axum
- **serde**: Zero-copy deserialization for high-performance message parsing

## Building and Running

### Prerequisites

- Rust 1.70+ (edition 2021)
- Cargo

### Development Build

```bash
# Clone the repository
git clone <repository-url>
cd based.mobi

# Build in debug mode
cargo build

# Run with debug logging
RUST_LOG=debug cargo run
```

### Release Build

```bash
# Optimized build
cargo build --release

# Run release binary
cargo run --release
```

### Environment Variables

- `PORT`: Server port (default: `8000`)
- `RUST_LOG`: Logging level (default: `info`)

### Running the Server

```bash
# Default port 8000
cargo run --release

# Custom port
PORT=3000 cargo run --release

# With debug logging
RUST_LOG=debug cargo run --release
```

The server will:
1. Initialize tracing/logging
2. Create shared `AppState` with `GameManager` and `StateManager`
3. Spawn background timer task (runs every 2 seconds)
4. Start Axum server on `0.0.0.0:PORT`
5. Serve static files from `static/` directory
6. Accept WebSocket connections at `/ws`

## Code Organization

### Module Responsibilities

**`chess_game`**: Core chess logic
- `ChessGame` struct holds game state (position, players, timers, draw/resign state)
- Move validation via `shakmaty::Chess::play()`
- Timer management: tracks elapsed time, applies increments, detects timeouts
- Position signature tracking for threefold repetition detection

**`game_manager`**: Game lifecycle and matchmaking
- `GameManager` maintains `HashMap<String, ChessGame>` for active games
- Room system: password-protected rooms, spectator control
- Lobby queue: simple FIFO queue for automatic matchmaking
- Spectator tracking: separate from player tracking

**`handlers`**: Request processing
- Each handler implements domain-specific logic (moves, draws, rooms)
- Handlers receive `HandlerContext` with access to `GameManager` and `StateManager`
- Return `Result<(), String>` for error handling
- Broadcast game state updates after mutations

**`messages`**: WebSocket protocol
- `ClientMessage`: Enum of all incoming message types
- `ServerMessage`: Enum of all outgoing message types
- JSON serialization via `serde`

**`connection`**: WebSocket lifecycle
- `handle_socket()`: Sets up per-connection send/receive tasks
- `cleanup_disconnect()`: Removes player from games, notifies others
- Player ID generation via `uuid::Uuid`

### State Management

**`AppState`** (shared across all connections):
```rust
pub struct AppState {
    pub game_manager: Arc<RwLock<GameManager>>,
    pub state_manager: Arc<RwLock<StateManager>>,
}
```

**`GameManager`** (game state):
- `games: HashMap<String, ChessGame>` - active games by game_id
- `player_to_game: HashMap<String, String>` - player_id -> game_id mapping
- `lobby_queue: Vec<String>` - matchmaking queue
- `spectators: HashMap<String, HashSet<String>>` - game_id -> spectator_ids

**`StateManager`** (connection state):
- `connections: HashMap<String, Sender>` - player_id -> WebSocket sender
- `player_names: HashMap<String, String>` - player_id -> display name

### Locking Strategy

- **Read locks** (`read().await`): For queries that don't modify state
- **Write locks** (`write().await`): For mutations (creating games, making moves)
- **Lock scope**: Locks held only for the duration of the operation
- **Deadlock prevention**: Consistent lock ordering (GameManager before StateManager when both needed)

## Chess Rules Implementation

The server implements FIDE-compliant chess rules using `shakmaty`:

### Move Validation

- All moves validated server-side via `shakmaty::Chess::play()`
- Illegal moves rejected with error message
- Legal move generation for piece selection UI

### Game End Conditions

- **Checkmate**: Detected via `shakmaty::Position::is_checkmate()`
- **Stalemate**: Detected via `shakmaty::Position::is_stalemate()`
- **Insufficient material**: Detected via `shakmaty::Position::is_insufficient_material()`
- **Seventy-five-move rule**: Automatic draw after 150 half-moves without pawn move or capture
- **Threefold repetition**: Claimable draw (player must claim on their turn)
- **Fifty-move rule**: Claimable draw (player must claim on their turn)

### Draw Claims

- **Threefold repetition**: Player claims when same position occurs 3 times (tracked via position signatures)
- **Fifty-move rule**: Player claims after 50 moves without pawn move or capture
- **Draw by agreement**: Either player can offer, opponent can accept
- **Seventy-five-move rule**: Automatic (no claim needed)

### Time Controls

Supported presets:
- **Bullet**: 2 minutes + 1 second increment
- **Blitz**: 3 minutes + 2 seconds increment
- **Rapid**: 15 minutes + 10 seconds increment
- **Classical**: 60 minutes + 30 seconds increment

Timer logic:
- Server maintains authoritative clocks
- Background task updates every 2 seconds
- Time decremented on move completion
- Increment applied after move
- Timeout detection: game ends when clock hits zero
- Insufficient material exception: draw even if player flags

## WebSocket Protocol

### Message Format

All messages are JSON objects with a `type` field:

```json
{
  "type": "make_move",
  "move": "e2e4"
}
```

### Client Messages (Incoming)

| Type | Fields | Description |
|------|--------|-------------|
| `make_move` | `move: String` (UCI) | Execute a move |
| `get_game_state` | - | Request current game state |
| `get_legal_moves` | `square: String` | Get legal moves for a square |
| `create_room` | `password?: String`, `allow_spectators?: bool`, `time_control?: String`, `play_against_bot?: bool` | Create a new game room |
| `join_room` | `room_id: String`, `password?: String` | Join an existing room |
| `leave_room` | - | Leave current room |
| `offer_draw` | - | Offer a draw |
| `accept_draw` | - | Accept pending draw offer |
| `claim_draw` | `reason: "threefold_repetition" \| "fifty_moves"` | Claim a draw by rule |
| `resign` | - | Resign the game |
| `request_rematch` | - | Request a rematch |
| `accept_rematch` | - | Accept rematch request |
| `set_player_name` | `name: String` | Set display name |
| `join_queue` | - | Join matchmaking queue |
| `get_games_list` | - | Get list of active games |
| `spectate_game` | `game_id: String` | Join as spectator |

### Server Messages (Outgoing)

| Type | Fields | Description |
|------|--------|-------------|
| `connected` | `player_id`, `player_name` | Sent on connection |
| `game_state` | `board`, `turn`, `fen`, `is_game_over`, `result`, etc. | Full game state |
| `move_made` | `move_uci`, `player_name`, `player_color` | Move executed |
| `legal_moves` | `square`, `valid_squares`, `legal_moves` | Legal moves for square |
| `matched` | `game_id`, `room_id`, `white_player`, `black_player`, `color` | Matched in queue |
| `error` | `message` | Error occurred |

See `src/messages/client.rs` and `src/messages/server.rs` for complete message definitions.

## Contributing

### Development Workflow

1. **Fork and clone** the repository
2. **Create a feature branch**: `git checkout -b feature/your-feature`
3. **Make changes** following the existing code style
4. **Test locally**: Run `cargo run` and test in browser
5. **Commit**: Use clear commit messages
6. **Push and open PR**: Describe changes and rationale

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Use `cargo clippy` to catch common issues
- Prefer explicit error handling over `unwrap()` in production paths
- Use `tracing::debug!` / `tracing::info!` for logging
- Document public APIs with `///` doc comments

### Areas for Contribution

- **Chess rules**: Additional FIDE rule implementations, edge cases
- **Performance**: Optimize lock contention, reduce allocations
- **Testing**: Unit tests for game logic, integration tests for WebSocket flow
- **Features**: New time controls, tournament modes, PGN export
- **Documentation**: Code comments, architecture diagrams, protocol docs

### Testing

Currently, the project lacks automated tests. Contributions adding tests are welcome:

- **Unit tests**: Test individual functions (move validation, timer logic, draw claims)
- **Integration tests**: Test WebSocket message flow end-to-end
- **Property tests**: Use `proptest` to generate random valid moves

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_move_validation() {
        // Test move validation logic
    }
}
```

## Deployment

### Docker

A `Dockerfile` is provided for containerized deployment:

```bash
# Build image
docker build -t chess-server .

# Run container
docker run -p 8000:8000 chess-server
```

### Production Considerations

- **Reverse proxy**: Run behind nginx/Caddy for HTTPS termination
- **Process manager**: Use systemd, supervisor, or Docker Compose
- **Logging**: Configure `RUST_LOG` for appropriate log level
- **Resource limits**: Monitor memory usage (games are in-memory)
- **Graceful shutdown**: Implement signal handling for clean disconnects

### Static Files

The server serves static files from the `static/` directory:
- `index.html` - Main game interface
- `replay.html` - Game replay viewer
- `about.html` - About page
- `app.js` - Client-side JavaScript
- `style.css` - Stylesheet
- `images/` - Piece images
- `sounds/` - Sound effects
- Stockfish WASM files for browser-based engine

All static files are served with cache-busting query parameters injected at runtime.

## License

[Specify your license here]

## Acknowledgments

- **shakmaty**: Chess rules engine by niklasf
- **axum**: Web framework by tokio-rs
- **Stockfish**: Chess engine (WASM build by niklasf)
