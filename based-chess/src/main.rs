mod chess_game;
mod connection;
mod game_manager;
mod handlers;
mod messages;
mod models;
mod services;
mod utils;
mod websocket;

use axum::{
    extract::ws::WebSocketUpgrade,
    http::{HeaderValue, StatusCode},
    middleware,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

use connection::AppState;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let app_state = AppState {
        game_manager: std::sync::Arc::new(tokio::sync::RwLock::new(game_manager::GameManager::new())),
        state_manager: std::sync::Arc::new(tokio::sync::RwLock::new(models::StateManager::new())),
    };

    // Spawn background timer task
    // Update every 2 seconds instead of every second to reduce interruptions
    // Client-side timer handles smooth countdown, server only needs to sync periodically
    let game_manager_clone = app_state.game_manager.clone();
    let state_manager_clone = app_state.state_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            interval.tick().await;
            
            let games_to_update = {
                let mut gm = game_manager_clone.write().await;
                gm.check_timers()
            };
            
            // Send game state updates to all players/spectators in games that need updates
            for game_id in games_to_update {
                let _ = crate::handlers::helpers::send_game_state_to_all(
                    &game_id,
                    game_manager_clone.clone(),
                    state_manager_clone.clone(),
                ).await;
            }
        }
    });

    // Build router with headers for SharedArrayBuffer support (required for Stockfish WASM)
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/replay.html", get(replay_handler))
        .route("/about.html", get(about_handler))
        .route("/piece-images", get(piece_images_handler))
        .route("/ws", get(websocket_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn(add_shared_array_buffer_headers))
        .with_state(app_state);

    // Get port from environment variable or default to 8000
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);
    
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();
    
    tracing::info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler() -> impl IntoResponse {
    match tokio::fs::read_to_string("static/index.html").await {
        Ok(html) => {
            let html_with_cache_busting = utils::cache_busting::inject_cache_busting(&html, "static");
            Html(html_with_cache_busting).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn replay_handler() -> impl IntoResponse {
    match tokio::fs::read_to_string("static/replay.html").await {
        Ok(html) => {
            let html_with_cache_busting = utils::cache_busting::inject_cache_busting(&html, "static");
            Html(html_with_cache_busting).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn about_handler() -> impl IntoResponse {
    match tokio::fs::read_to_string("static/about.html").await {
        Ok(html) => {
            let html_with_cache_busting = utils::cache_busting::inject_cache_busting(&html, "static");
            Html(html_with_cache_busting).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn piece_images_handler() -> impl IntoResponse {
    match utils::piece_images::load_piece_images("static/images").await {
        Ok(images) => Json(images).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load images").into_response(),
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| connection::handle_socket(socket, state))
}

// Middleware to add headers required for SharedArrayBuffer (Stockfish WASM)
// Safari requires "require-corp" (not "credentialless") for SharedArrayBuffer support
// All external resources must be self-hosted to work with require-corp
async fn add_shared_array_buffer_headers(
    request: axum::extract::Request,
    next: middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    // Use require-corp for Safari compatibility (Safari requires this for SharedArrayBuffer)
    // All resources are now self-hosted (Chess.js, Stockfish, etc.) so require-corp works
    headers.insert(
        axum::http::header::HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    );
    response
}
