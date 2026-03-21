import { state } from './state.js';

/**
 * Initialize Stockfish engine
 */
export async function initializeStockfish() {
    if (state.stockfishEngine) {
        console.log('[Bot] ✅ Stockfish already initialized');
        return;
    }
    
    if (typeof Stockfish === 'undefined') {
        console.error('[Bot] ❌ Stockfish not loaded. Make sure stockfish.js is included.');
        return;
    }
    
    // Check if SharedArrayBuffer is available (required for Stockfish WASM)
    if (typeof SharedArrayBuffer === 'undefined') {
        console.error('[Bot] ❌ SharedArrayBuffer not available. Stockfish requires SharedArrayBuffer support.');
        const isSafari = /^((?!chrome|android).)*safari/i.test(navigator.userAgent);
        const isSecureContext = window.isSecureContext;
        const protocol = window.location.protocol;
        
        let errorMsg = 'Bot mode requires SharedArrayBuffer support.';
        if (isSafari) {
            if (!isSecureContext || protocol !== 'https:') {
                errorMsg += ' Safari requires HTTPS. Please access the site over HTTPS (not HTTP).';
            } else {
                errorMsg += ' Please ensure you\'re using Safari 15.2+ and the server is configured with proper security headers.';
            }
        } else {
            errorMsg += ' Please try using Chrome, Firefox, or ensure you\'re on a secure context (HTTPS).';
        }
        console.error('[Bot] ⚠️', errorMsg);
        console.error('[Bot] 🔍 Debug info:', { isSafari, isSecureContext, protocol });
        
        // Show error message in bot status element
        const botStatus = document.getElementById('bot-status');
        if (botStatus) {
            botStatus.textContent = '❌ Bot unavailable (SharedArrayBuffer not supported)';
            botStatus.style.color = '#dc3545';
            botStatus.style.display = 'block';
        }
        
        // Disable bot mode
        const botCheckbox = document.getElementById('play-against-bot-checkbox');
        if (botCheckbox) {
            botCheckbox.checked = false;
            state.playAgainstBot = false;
            const botDifficultyContainer = document.getElementById('bot-difficulty-container');
            if (botDifficultyContainer) botDifficultyContainer.style.display = 'none';
        }
        return;
    }
    
    console.log('[Bot] 🚀 Initializing Stockfish engine...');
    
    try {
        // Stockfish() returns a module factory function
        const StockfishModule = Stockfish();
        
        // The module factory might return a Promise or the module directly
        let stockfishModule;
        if (StockfishModule && typeof StockfishModule.then === 'function') {
            stockfishModule = await StockfishModule;
        } else {
            stockfishModule = StockfishModule;
        }
        
        // Wait for the module to be ready if it has a ready Promise
        if (stockfishModule && stockfishModule.ready) {
            await stockfishModule.ready;
        }
        
        // Log the module structure for debugging
        console.log('[Bot] 🔍 Stockfish module structure:', {
            hasAddMessageListener: typeof stockfishModule?.addMessageListener === 'function',
            hasPostMessage: typeof stockfishModule?.postMessage === 'function',
            keys: stockfishModule ? Object.keys(stockfishModule).slice(0, 10) : []
        });
        
        // The module should now be ready - check if methods exist
        if (!stockfishModule || typeof stockfishModule.addMessageListener !== 'function') {
            throw new Error('Stockfish module does not have addMessageListener method. Module: ' + JSON.stringify(Object.keys(stockfishModule || {})));
        }
        
        state.stockfishEngine = stockfishModule;
        
        // Set up a single global message handler
        state.stockfishEngine.addMessageListener((line) => {
            const trimmedLine = line.trim();
            console.log('[Bot] 📨 Stockfish message:', trimmedLine);
            
            // Handle bestmove responses
            if (trimmedLine.startsWith('bestmove') && state.stockfishCallback) {
                const parts = trimmedLine.split(/\s+/);
                console.log('[Bot] 📊 Parsing bestmove, parts:', parts);
                const callback = state.stockfishCallback;
                state.stockfishCallback = null; // Clear callback
                
                if (parts.length > 1 && parts[1] !== 'none') {
                    const bestMove = parts[1];
                    console.log('[Bot] 🎯 Stockfish best move:', bestMove);
                    callback(bestMove);
                } else {
                    // No legal moves (checkmate/stalemate)
                    console.log('[Bot] ❌ No legal moves available');
                    callback(null);
                }
            }
        });
        
        // Configure Stockfish
        state.stockfishEngine.postMessage('uci');
        state.stockfishEngine.postMessage(`setoption name Skill Level value ${state.botLevel}`);
        state.stockfishEngine.postMessage('isready');
        
        console.log('[Bot] ✅ Stockfish engine initialized');
    } catch (error) {
        console.error('[Bot] ❌ Error initializing Stockfish:', error);
        state.stockfishEngine = null;
    }
}

/**
 * Cleanup Stockfish engine
 */
export function cleanupStockfish() {
    if (state.stockfishEngine) {
        console.log('[Bot] 🧹 Cleaning up Stockfish engine');
        state.stockfishEngine.terminate();
        state.stockfishEngine = null;
    }
}

/**
 * Generate a move using Stockfish engine
 * @param {string} fen - FEN position string
 * @param {Function} callback - Callback function to receive the move
 */
export function generateStockfishMove(fen, callback) {
    if (!state.stockfishEngine) {
        console.error('[Bot] ❌ Stockfish engine not initialized');
        callback(null);
        return;
    }
    
    // If there's already a pending request, cancel it with fallback
    if (state.stockfishCallback) {
        console.log('[Bot] ⚠️ Cancelling previous Stockfish request');
        const oldCallback = state.stockfishCallback;
        state.stockfishCallback = null;
        // Fallback to random move for the old request
        try {
            const chess = new Chess(fen);
            const moves = chess.moves({ verbose: true });
            if (moves.length > 0) {
                const randomMove = moves[Math.floor(Math.random() * moves.length)];
                let moveUci = randomMove.from + randomMove.to;
                if (randomMove.promotion) {
                    moveUci += randomMove.promotion.toLowerCase();
                }
                oldCallback(moveUci);
            } else {
                oldCallback(null);
            }
        } catch (error) {
            oldCallback(null);
        }
    }
    
    console.log('[Bot] 🎮 Starting Stockfish analysis for FEN:', fen);
    
    // Set the callback for this request
    state.stockfishCallback = callback;
    
    // Set position and start analysis
    console.log('[Bot] 📤 Sending position to Stockfish');
    state.stockfishEngine.postMessage(`position fen ${fen}`);
    console.log('[Bot] 📤 Sending go command to Stockfish');
    state.stockfishEngine.postMessage('go depth 10'); // Search to depth 10 for reasonable speed
    
    // Timeout fallback (in case Stockfish doesn't respond)
    setTimeout(() => {
        if (state.stockfishCallback === callback) {
            // This request is still pending
            state.stockfishCallback = null;
            console.error('[Bot] ❌ Stockfish timeout after 5 seconds - falling back to random move');
            // Fallback to random move
            if (typeof Chess === 'undefined') {
                console.error('[Bot] ❌ Chess.js not available for fallback');
                callback(null);
                return;
            }
            
            try {
                const chess = new Chess(fen);
                const moves = chess.moves({ verbose: true });
                if (moves.length > 0) {
                    const randomMove = moves[Math.floor(Math.random() * moves.length)];
                    let moveUci = randomMove.from + randomMove.to;
                    if (randomMove.promotion) {
                        moveUci += randomMove.promotion.toLowerCase();
                    }
                    console.log('[Bot] 🎲 Using fallback random move:', moveUci);
                    callback(moveUci);
                } else {
                    callback(null);
                }
            } catch (error) {
                console.error('[Bot] ❌ Error in fallback move generation:', error);
                callback(null);
            }
        }
    }, 5000); // 5 second timeout
}

/**
 * Send bot move to server via bot WebSocket
 * @param {string} moveUci - Move in UCI format
 */
export function sendBotMove(moveUci) {
    if (!state.botWs || state.botWs.readyState !== WebSocket.OPEN) {
        console.error('[Bot] ❌ Bot WebSocket not connected when trying to send move:', moveUci);
        return;
    }
    
    console.log('[Bot] 📤 Sending move to server:', moveUci);
    state.botWs.send(JSON.stringify({
        type: 'make_move',
        move: moveUci
    }));
}

/**
 * Connect bot WebSocket for bot mode
 */
export function connectBotWebSocket() {
    if (!state.playAgainstBot || state.botRoomId === null) {
        console.log('[Bot] ⏭️ Skipping bot WebSocket connection - bot mode disabled or no room ID');
        return;
    }
    
    console.log('[Bot] 🔌 Connecting bot WebSocket to room:', state.botRoomId);
    
    // Close existing bot connection if any
    if (state.botWs) {
        console.log('[Bot] 🧹 Closing existing bot WebSocket connection');
        state.botWs.close();
        state.botWs = null;
    }
    
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    console.log('[Bot] 📡 Creating bot WebSocket:', wsUrl);
    state.botWs = new WebSocket(wsUrl);
    
    state.botWs.onopen = () => {
        console.log('[Bot] ✅ Bot WebSocket connected');
    };
    
    state.botWs.onmessage = (event) => {
        try {
            const message = JSON.parse(event.data);
            console.log('[Bot] 📨 Bot WebSocket message received:', message.type);
            handleBotMessage(message);
        } catch (error) {
            console.error('[Bot] ❌ Error parsing bot WebSocket message:', error, event.data);
        }
    };
    
    state.botWs.onclose = () => {
        console.log('[Bot] 🔌 Bot WebSocket closed');
        state.botWs = null;
        // Reconnect after 2 seconds if still in bot mode
        if (state.playAgainstBot && state.botRoomId) {
            console.log('[Bot] 🔄 Reconnecting bot WebSocket in 2 seconds...');
            setTimeout(connectBotWebSocket, 2000);
        }
    };
    
    state.botWs.onerror = (error) => {
        console.error('[Bot] ❌ Bot WebSocket error:', error);
    };
}

/**
 * Process bot game state - make move if it's bot's turn
 * @param {Object} message - Game state message
 */
export async function processBotGameState(message) {
    console.log('[Bot] 🔍 Processing game state:', {
        playAgainstBot: state.playAgainstBot,
        botColor: state.botColor,
        botThinking: state.botThinking,
        gameStarted: message.game_started,
        isGameOver: message.is_game_over,
        turn: message.turn,
        fen: message.fen
    });
    
    if (!state.playAgainstBot) {
        console.log('[Bot] ⏭️ Skipping - bot mode not enabled');
        return;
    }
    
    if (!state.botColor) {
        console.log('[Bot] ⏭️ Skipping - bot color not set yet');
        return;
    }
    
    // Check if game is started and it's bot's turn
    if (!message.game_started) {
        console.log('[Bot] ⏭️ Skipping - game not started');
        return;
    }
    
    if (message.is_game_over) {
        console.log('[Bot] ⏭️ Skipping - game is over');
        return;
    }
    
    // Check if it's bot's turn (handle null/undefined turn)
    if (!message.turn || message.turn !== state.botColor) {
        console.log(`[Bot] ⏭️ Skipping - not bot's turn (turn: ${message.turn}, bot: ${state.botColor})`);
        // If turn changed to opponent and bot was thinking, reset botThinking
        if (state.botThinking && message.turn !== state.botColor) {
            console.log('[Bot] 🔄 Turn changed to opponent, resetting botThinking (safety check)');
            state.botThinking = false;
        }
        // Update last processed turn only if it actually changed
        if (state.lastProcessedTurn !== message.turn) {
            state.lastProcessedTurn = message.turn;
        }
        return;
    }
    
    // If it's bot's turn again and we were thinking, it means opponent moved
    if (state.botThinking && message.turn === state.botColor && state.lastProcessedTurn !== message.turn) {
        console.log('[Bot] 🔄 Turn changed back to bot (opponent moved), resetting botThinking');
        state.botThinking = false;
    }
    
    // Check if we've already processed this turn
    if (state.lastProcessedTurn === message.turn && state.botThinking) {
        console.log(`[Bot] ⏭️ Skipping - already processing this turn (turn: ${message.turn})`);
        return;
    }
    
    if (state.botThinking) {
        console.log('[Bot] ⏭️ Skipping - bot already thinking');
        return;
    }
    
    // Set botThinking flag
    state.botThinking = true;
    state.lastProcessedTurn = message.turn;
    
    // Get FEN from game state
    const fen = message.fen || 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
    
    console.log('[Bot] 🤖 Bot\'s turn! Getting Stockfish move for FEN:', fen);
    
    // Ensure Stockfish is initialized
    if (!state.stockfishEngine) {
        console.log('[Bot] 🔄 Stockfish not initialized, initializing now...');
        await initializeStockfish();
        // If initialization failed, use fallback
        if (!state.stockfishEngine) {
            console.error('[Bot] ❌ Stockfish initialization failed, using fallback');
            state.botThinking = false;
            // Fallback to random move
            let chessAvailable = typeof Chess !== 'undefined';
            if (!chessAvailable) {
                // Wait up to 1 second for Chess.js to load
                for (let i = 0; i < 10; i++) {
                    await new Promise(resolve => setTimeout(resolve, 100));
                    if (typeof Chess !== 'undefined') {
                        chessAvailable = true;
                        break;
                    }
                }
            }
            
            if (!chessAvailable) {
                console.error('[Bot] ❌ Chess.js not loaded - cannot generate fallback moves');
                state.botThinking = false;
                return;
            }
            
            try {
                const chess = new Chess(fen);
                const moves = chess.moves({ verbose: true });
                if (moves.length > 0) {
                    const randomMove = moves[Math.floor(Math.random() * moves.length)];
                    let moveUci = randomMove.from + randomMove.to;
                    if (randomMove.promotion) {
                        moveUci += randomMove.promotion.toLowerCase();
                    }
                    sendBotMove(moveUci);
                } else {
                    state.botThinking = false;
                }
            } catch (error) {
                console.error('[Bot] ❌ Error in fallback move generation:', error);
                state.botThinking = false;
            }
            return;
        }
    }
    
    // Generate Stockfish move with a small delay to simulate thinking
    setTimeout(async () => {
        generateStockfishMove(fen, (move) => {
            if (move) {
                sendBotMove(move);
            } else {
                console.error('[Bot] ❌ Failed to generate move');
                state.botThinking = false;
            }
        });
    }, 300); // Small delay before starting analysis
}

/**
 * Handle messages on bot WebSocket
 * @param {Object} message - Message from server
 */
export function handleBotMessage(message) {
    switch (message.type) {
        case 'connected':
            console.log('[Bot] ✅ Bot connected, player ID:', message.player_id);
            state.botPlayerId = message.player_id;
            // Set bot name to "Stockfish"
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                console.log('[Bot] 📤 Setting bot name to "Stockfish"');
                state.botWs.send(JSON.stringify({ 
                    type: 'set_player_name', 
                    name: 'Stockfish' 
                }));
            }
            // Join the room
            if (state.botRoomId && state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                const joinMsg = {
                    type: 'join_room',
                    room_id: state.botRoomId
                };
                if (state.botPassword) {
                    joinMsg.password = state.botPassword;
                }
                console.log('[Bot] 📤 Joining room:', state.botRoomId);
                state.botWs.send(JSON.stringify(joinMsg));
            }
            break;
        
        case 'room_joined':
            console.log('[Bot] ✅ Bot joined room');
            // Automatically set bot ready after a short delay
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    console.log('[Bot] 📤 Setting bot ready');
                    state.botWs.send(JSON.stringify({ type: 'player_ready' }));
                    // Request game state after setting ready
                    setTimeout(() => {
                        if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                            state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                        }
                    }, 500);
                }, 1000);
            }
            break;
        
        case 'game_state':
            // Update bot color from game state
            if (message.your_color && message.your_color !== 'spectator') {
                if (state.botColor !== message.your_color) {
                    console.log(`[Bot] 🎨 Bot color set to: ${message.your_color}`);
                    state.botColor = message.your_color;
                }
            }
            
            // Bot automatically requests rematch when game ends if rematch is available
            if (message.can_rematch && message.is_game_over) {
                const rematchRequested = message.rematch_requested_by === state.botPlayerId;
                const rematchRequestedByOpponent = message.rematch_requested_by && message.rematch_requested_by !== state.botPlayerId;
                
                // Check if rematch hasn't been requested yet
                if (!rematchRequested && !rematchRequestedByOpponent) {
                    console.log('[Bot] 🔄 Game ended, automatically requesting rematch...');
                    if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                        setTimeout(() => {
                            console.log('[Bot] 📤 Requesting rematch');
                            state.botWs.send(JSON.stringify({ type: 'request_rematch' }));
                        }, 1000); // Small delay before requesting
                    }
                }
            }
            
            // Check if a draw was offered to the bot
            if (message.draw_offered_by && message.draw_offered_by !== state.botPlayerId) {
                console.log('[Bot] 🤝 Draw offer detected, will reject by making a move');
            }
            
            // Skip timer-only updates when bot is thinking
            if (state.botThinking && message.turn === state.botColor && state.lastProcessedTurn === message.turn) {
                console.log('[Bot] ⏭️ Skipping timer update - bot is thinking on same turn');
                return;
            }
            
            // If turn changed, reset botThinking
            if (state.botThinking && message.turn !== state.botColor) {
                console.log('[Bot] 🔄 Turn changed to opponent, resetting botThinking');
                state.botThinking = false;
                state.lastProcessedTurn = message.turn;
            }
            
            // Process bot move if it's bot's turn
            processBotGameState(message);
            break;
        
        case 'game_started':
            console.log('[Bot] 🎮 Game started!');
            if (message.your_color && message.your_color !== 'spectator') {
                console.log(`[Bot] 🎨 Bot color: ${message.your_color}`);
                state.botColor = message.your_color;
            }
            // Request game state to check if it's bot's turn
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    console.log('[Bot] 📤 Requesting game state after game started');
                    state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                }, 500);
            }
            break;
        
        case 'move_made':
            console.log('[Bot] ✅ Move made:', message.move_uci);
            state.botThinking = false;
            state.lastProcessedTurn = null;
            // Request new game state
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                }, 500);
            }
            break;
        
        case 'error':
            console.error('[Bot] ❌ Server error:', message.message);
            state.botThinking = false;
            state.lastProcessedTurn = null;
            // Request game state to get current state
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                }, 500);
            }
            break;
        
        case 'draw_offered':
            console.log('[Bot] 🤝 Draw offered, rejecting...');
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                        state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                    }
                }, 500);
            }
            break;
        
        case 'rematch_requested':
            console.log('[Bot] 🔄 Rematch requested, accepting...');
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    console.log('[Bot] 📤 Accepting rematch');
                    state.botWs.send(JSON.stringify({ type: 'accept_rematch' }));
                }, 500);
            }
            break;
        
        case 'rematch_started':
            console.log('[Bot] 🔄 Rematch started');
            if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                setTimeout(() => {
                    console.log('[Bot] 📤 Setting bot ready for rematch');
                    state.botWs.send(JSON.stringify({ type: 'player_ready' }));
                    setTimeout(() => {
                        if (state.botWs && state.botWs.readyState === WebSocket.OPEN) {
                            state.botWs.send(JSON.stringify({ type: 'get_game_state' }));
                        }
                    }, 500);
                }, 1000);
            }
            break;
        
        default:
            break;
    }
}

