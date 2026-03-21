/**
 * Game Replay Module
 * Handles replaying saved chess games with controls
 */

import { getGame, deleteGame, getAllGames } from './gameStorage.js';

// Replay state
let gameData = null;
let currentGameId = null; // Store the current game ID for delete/download
let currentMoveIndex = -1; // -1 = start position, 0+ = move index
let chess = null;
let isPlaying = false;
let playInterval = null;
let pieceImageUrls = {};
let replayViewColor = 'white'; // 'white' or 'black' - which perspective we're viewing from
let stockfishEngine = null;
let stockfishAnalysisCallback = null;
let stockfishEvaluationCallback = null;
let moveAnalysis = null; // Store detailed move analysis
let moveEvaluations = null; // Store move evaluations
let analysisEnabled = true;

// Load piece images
async function loadPieceImages() {
    try {
        const response = await fetch('/piece-images');
        if (response.ok) {
            const data = await response.json();
            pieceImageUrls = data;
            
            Object.values(pieceImageUrls).forEach(dataUri => {
                const img = new Image();
                img.src = dataUri;
            });
        } else {
            // Fallback to static URLs
            pieceImageUrls = {
                'K': '/static/images/white_king.png',
                'Q': '/static/images/white_queen.png',
                'R': '/static/images/white_rook.png',
                'B': '/static/images/white_bishop.png',
                'N': '/static/images/white_knight.png',
                'P': '/static/images/white_pawn.png',
                'k': '/static/images/black_king.png',
                'q': '/static/images/black_queen.png',
                'r': '/static/images/black_rook.png',
                'b': '/static/images/black_bishop.png',
                'n': '/static/images/black_knight.png',
                'p': '/static/images/black_pawn.png'
            };
        }
    } catch (error) {
        console.error('Error loading piece images:', error);
        pieceImageUrls = {
            'K': '/static/images/white_king.png',
            'Q': '/static/images/white_queen.png',
            'R': '/static/images/white_rook.png',
            'B': '/static/images/white_bishop.png',
            'N': '/static/images/white_knight.png',
            'P': '/static/images/white_pawn.png',
            'k': '/static/images/black_king.png',
            'q': '/static/images/black_queen.png',
            'r': '/static/images/black_rook.png',
            'b': '/static/images/black_bishop.png',
            'n': '/static/images/black_knight.png',
            'p': '/static/images/black_pawn.png'
        };
    }
}

// Initialize Stockfish engine
async function initializeStockfish() {
    if (stockfishEngine) {
        console.log('[Replay] ✅ Stockfish already initialized');
        return;
    }
    
    if (typeof Stockfish === 'undefined') {
        console.error('[Replay] ❌ Stockfish not loaded');
        return;
    }
    
    if (typeof SharedArrayBuffer === 'undefined') {
        console.error('[Replay] ❌ SharedArrayBuffer not available');
        return;
    }
    
    console.log('[Replay] 🚀 Initializing Stockfish engine...');
    
    try {
        const StockfishModule = Stockfish();
        let stockfishModule;
        if (StockfishModule && typeof StockfishModule.then === 'function') {
            stockfishModule = await StockfishModule;
        } else {
            stockfishModule = StockfishModule;
        }
        
        if (stockfishModule && stockfishModule.ready) {
            await stockfishModule.ready;
        }
        
        if (!stockfishModule || typeof stockfishModule.addMessageListener !== 'function') {
            throw new Error('Stockfish module does not have addMessageListener method');
        }
        
        stockfishEngine = stockfishModule;
        
        stockfishEngine.addMessageListener((line) => {
            const trimmedLine = line.trim();
            
            // Handle evaluation responses
            if (stockfishEvaluationCallback) {
                if (trimmedLine.startsWith('info')) {
                    const scoreMatch = trimmedLine.match(/score (cp|mate) (-?\d+)/);
                    if (scoreMatch) {
                        const scoreType = scoreMatch[1];
                        const scoreValue = parseInt(scoreMatch[2]);
                        let centipawns = scoreValue;
                        if (scoreType === 'mate') {
                            centipawns = scoreValue > 0 ? 10000 - scoreValue : -10000 - scoreValue;
                        }
                        stockfishEvaluationCallback.lastEval = centipawns;
                    }
                }
                if (trimmedLine.startsWith('bestmove')) {
                    const callback = stockfishEvaluationCallback;
                    const lastEval = callback.lastEval !== undefined ? callback.lastEval : 0;
                    stockfishEvaluationCallback = null;
                    if (typeof callback === 'function') {
                        callback(lastEval);
                    }
                }
            }
            
            // Handle analysis responses
            if (stockfishAnalysisCallback) {
                if (trimmedLine.startsWith('info')) {
                    const scoreMatch = trimmedLine.match(/score (cp|mate) (-?\d+)/);
                    if (scoreMatch) {
                        const scoreType = scoreMatch[1];
                        const scoreValue = parseInt(scoreMatch[2]);
                        let centipawns = scoreValue;
                        if (scoreType === 'mate') {
                            centipawns = scoreValue > 0 ? 10000 - scoreValue : -10000 - scoreValue;
                        }
                        stockfishAnalysisCallback.lastEval = centipawns;
                    }
                }
                if (trimmedLine.startsWith('bestmove')) {
                    const parts = trimmedLine.split(/\s+/);
                    const callback = stockfishAnalysisCallback;
                    const lastEval = callback.lastEval !== undefined ? callback.lastEval : 0;
                    stockfishAnalysisCallback = null;
                    if (typeof callback === 'function') {
                        const bestMove = (parts.length > 1 && parts[1] !== 'none') ? parts[1] : null;
                        callback(bestMove, lastEval);
                    }
                }
            }
        });
        
        stockfishEngine.postMessage('uci');
        stockfishEngine.postMessage('isready');
        
        console.log('[Replay] ✅ Stockfish engine initialized');
        
        // Analyze the game after initialization
        if (gameData && gameData.moveSanHistory) {
            analyzeGameMoves(gameData.moveSanHistory);
        }
    } catch (error) {
        console.error('[Replay] ❌ Error initializing Stockfish:', error);
        stockfishEngine = null;
    }
}

// Get position evaluation from Stockfish
function getStockfishEvaluation(fen, callback) {
    if (!stockfishEngine) {
        callback(0);
        return;
    }
    
    if (stockfishEvaluationCallback) {
        const oldCallback = stockfishEvaluationCallback;
        stockfishEvaluationCallback = null;
        if (typeof oldCallback === 'function') {
            const lastEval = oldCallback.lastEval !== undefined ? oldCallback.lastEval : 0;
            oldCallback(lastEval);
        }
    }
    
    stockfishEvaluationCallback = callback;
    stockfishEvaluationCallback.lastEval = 0;
    
    stockfishEngine.postMessage(`position fen ${fen}`);
    stockfishEngine.postMessage('go depth 8');
    
    setTimeout(() => {
        if (stockfishEvaluationCallback === callback) {
            stockfishEvaluationCallback = null;
            const lastEval = callback.lastEval !== undefined ? callback.lastEval : 0;
            callback(lastEval);
        }
    }, 3000);
}

// Get best move and evaluation from Stockfish
function getStockfishBestMoveAndEvaluation(fen, callback) {
    if (!stockfishEngine) {
        callback(null, 0);
        return;
    }
    
    if (stockfishAnalysisCallback) {
        const oldCallback = stockfishAnalysisCallback;
        stockfishAnalysisCallback = null;
        if (typeof oldCallback === 'function') {
            const lastEval = oldCallback.lastEval !== undefined ? oldCallback.lastEval : 0;
            oldCallback(null, lastEval);
        }
    }
    
    stockfishAnalysisCallback = callback;
    stockfishAnalysisCallback.lastEval = 0;
    
    stockfishEngine.postMessage(`position fen ${fen}`);
    stockfishEngine.postMessage('go depth 12');
    
    setTimeout(() => {
        if (stockfishAnalysisCallback === callback) {
            stockfishAnalysisCallback = null;
            const lastEval = callback.lastEval !== undefined ? callback.lastEval : 0;
            callback(null, lastEval);
        }
    }, 5000);
}

// Get move quality annotation
function getMoveQuality(evalDiff) {
    if (evalDiff < 10) return 'best';
    if (evalDiff < 30) return 'excellent';
    if (evalDiff < 60) return 'good';
    if (evalDiff < 100) return 'inaccuracy';
    if (evalDiff < 200) return 'mistake';
    return 'blunder';
}

// Get annotation symbol
function getMoveAnnotation(quality) {
    switch (quality) {
        case 'best': return '✓';
        case 'excellent': return '!!';
        case 'good': return '!';
        case 'inaccuracy': return '?!';
        case 'mistake': return '?';
        case 'blunder': return '??';
        default: return '';
    }
}

// Analyze all moves in the game
async function analyzeGameMoves(moveSanHistory, initialFen = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1') {
    if (!stockfishEngine || !moveSanHistory || moveSanHistory.length === 0) {
        return;
    }
    
    console.log('[Replay] 🎯 Starting move analysis for', moveSanHistory.length, 'moves');
    
    const analysis = {};
    const evaluations = {};
    const chess = new Chess(initialFen);
    
    // Evaluate initial position
    await new Promise((resolve) => {
        getStockfishBestMoveAndEvaluation(chess.fen(), (bestMove, evaluation) => {
            evaluations[-1] = evaluation;
            resolve();
        });
    });
    
    // Analyze each move
    for (let i = 0; i < moveSanHistory.length; i++) {
        const san = moveSanHistory[i];
        if (!san) continue;
        
        try {
            const isWhite = i % 2 === 0;
            const beforeEval = evaluations[i - 1] !== undefined ? evaluations[i - 1] : evaluations[-1];
            const fenBefore = chess.fen();
            
            let bestMoveUci = null;
            let bestMoveEval = beforeEval;
            let bestMoveSan = null;
            
            await new Promise((resolve) => {
                getStockfishBestMoveAndEvaluation(fenBefore, (bestMove, evaluation) => {
                    bestMoveUci = bestMove;
                    bestMoveEval = evaluation;
                    resolve();
                });
            });
            
            const move = chess.move(san);
            if (!move) continue;
            
            let afterEval = beforeEval;
            await new Promise((resolve) => {
                getStockfishEvaluation(chess.fen(), (evaluation) => {
                    afterEval = evaluation;
                    evaluations[i] = evaluation;
                    resolve();
                });
            });
            
            const playedMoveUci = move.from + move.to + (move.promotion ? move.promotion.toLowerCase() : '');
            
            let evalDiff = 0;
            if (bestMoveUci && bestMoveUci !== playedMoveUci) {
                try {
                    const chessBest = new Chess(fenBefore);
                    const bestMoveObj = chessBest.move({
                        from: bestMoveUci.substring(0, 2),
                        to: bestMoveUci.substring(2, 4),
                        promotion: bestMoveUci.length > 4 ? bestMoveUci[4] : undefined
                    });
                    
                    if (bestMoveObj) {
                        bestMoveSan = bestMoveObj.san;
                        await new Promise((resolve) => {
                            getStockfishEvaluation(chessBest.fen(), (evalAfterBest) => {
                                if (isWhite) {
                                    evalDiff = Math.abs(afterEval - evalAfterBest);
                                } else {
                                    const playedChange = beforeEval - afterEval;
                                    const bestChange = beforeEval - evalAfterBest;
                                    evalDiff = Math.abs(playedChange - bestChange);
                                }
                                resolve();
                            });
                        });
                    }
                } catch (e) {
                    console.warn(`[Replay] Could not analyze best move for move ${i}:`, e);
                }
            }
            
            const quality = getMoveQuality(evalDiff);
            
            analysis[i] = {
                beforeEval: beforeEval,
                afterEval: afterEval,
                bestMove: bestMoveUci,
                bestMoveSan: bestMoveSan,
                playedMove: playedMoveUci,
                playedMoveSan: san,
                quality: quality,
                evalDiff: evalDiff,
                isWhite: isWhite
            };
            
            await new Promise(resolve => setTimeout(resolve, 50));
        } catch (error) {
            console.error(`[Replay] ❌ Error analyzing move ${i}:`, error);
            const beforeEval = evaluations[i - 1] || evaluations[-1] || 0;
            evaluations[i] = beforeEval;
            analysis[i] = {
                beforeEval: beforeEval,
                afterEval: beforeEval,
                bestMove: null,
                bestMoveSan: null,
                playedMove: null,
                playedMoveSan: san,
                quality: 'good',
                evalDiff: 0,
                isWhite: i % 2 === 0
            };
        }
    }
    
    moveAnalysis = analysis;
    moveEvaluations = evaluations;
    console.log('[Replay] ✅ Completed move analysis');
    
    // Update move notation display
    updateMoveNotation();
}

// Create chessboard
function createChessboard() {
    const board = document.getElementById('chessboard');
    board.innerHTML = '';
    
    for (let row = 0; row < 8; row++) {
        for (let col = 0; col < 8; col++) {
            const square = document.createElement('div');
            square.className = 'square';
            square.classList.add((row + col) % 2 === 0 ? 'light' : 'dark');
            square.dataset.row = row;
            square.dataset.col = col;
            
            const label = document.createElement('span');
            label.className = 'square-label';
            square.appendChild(label);
            
            board.appendChild(square);
        }
    }
    
    updateSquareLabels();
}

// Update square labels (a-h, 1-8) based on view
function updateSquareLabels() {
    const squares = document.querySelectorAll('.square');
    squares.forEach(square => {
        const row = parseInt(square.dataset.row);
        const col = parseInt(square.dataset.col);
        
        // Calculate what rank/file this square represents from the current view
        // Square row 0 = rank 8 from white's view, row 7 = rank 1 from white's view
        let displayRow, displayCol;
        if (replayViewColor === 'white') {
            displayRow = row; // row 0 = rank 8 (top), row 7 = rank 1 (bottom)
            displayCol = col; // col 0 = file a (left), col 7 = file h (right)
        } else {
            displayRow = 7 - row; // flipped: row 0 becomes rank 1 (bottom), row 7 becomes rank 8 (top)
            displayCol = 7 - col; // flipped: col 0 becomes file h (right), col 7 becomes file a (left)
        }
        
        // Calculate actual rank (1-8) and file (a-h) for display
        const rank = 8 - displayRow; // displayRow 0 = rank 8, displayRow 7 = rank 1
        const file = String.fromCharCode('a'.charCodeAt(0) + displayCol);
        
        const label = square.querySelector('.square-label');
        if (!label) return;
        
        // Show file labels (a-h) on bottom rank (rank 1) from current view
        // This is when displayRow === 7, which means row === 0 for white view, row === 7 for black view
        if (displayRow === 7) {
            label.textContent = file;
            label.style.display = 'block';
        }
        // Show rank labels (1-8) on left file (file a) from current view
        // This is when displayCol === 0, which means col === 0 for white view, col === 7 for black view
        else if (displayCol === 0) {
            label.textContent = rank;
            label.style.display = 'block';
        } else {
            label.style.display = 'none';
        }
    });
}

// Get square name from row and col
function getSquareName(row, col) {
    const displayRow = 7 - row;
    const file = String.fromCharCode('a'.charCodeAt(0) + col);
    const rank = displayRow + 1;
    return file + rank;
}

// Get square element by name
function getSquareElement(squareName) {
    const file = squareName[0].charCodeAt(0) - 'a'.charCodeAt(0);
    const rank = parseInt(squareName[1]) - 1;
    const row = 7 - rank;
    const col = file;
    return document.querySelector(`.square[data-row="${row}"][data-col="${col}"]`);
}

// Render board from FEN
function renderBoard(fen) {
    if (!fen) return;
    
    const chessInstance = new Chess(fen);
    const board = chessInstance.board();
    
    // Clear all pieces
    const squares = document.querySelectorAll('.square');
    squares.forEach(square => {
        const pieceImg = square.querySelector('.piece-img');
        if (pieceImg) {
            pieceImg.remove();
        }
    });
    
    // Place pieces
    // Board array from chess.js: row 0 = rank 8, row 7 = rank 1
    for (let row = 0; row < 8; row++) {
        for (let col = 0; col < 8; col++) {
            const piece = board[row][col];
            if (piece) {
                // Calculate which square element to use based on view
                // squares are stored with row 0 at top (which is rank 8 from white's view)
                let targetRow, targetCol;
                if (replayViewColor === 'white') {
                    // White view: row 0 = rank 8 (top), row 7 = rank 1 (bottom)
                    targetRow = row;
                    targetCol = col;
                } else {
                    // Black view: flip both row and col
                    // row 7 becomes 0, row 0 becomes 7
                    targetRow = 7 - row;
                    targetCol = 7 - col;
                }
                
                const square = document.querySelector(`.square[data-row="${targetRow}"][data-col="${targetCol}"]`);
                if (square) {
                    const pieceImg = document.createElement('img');
                    pieceImg.className = 'piece-img';
                    const pieceChar = piece.color === 'w' ? piece.type.toUpperCase() : piece.type.toLowerCase();
                    pieceImg.src = pieceImageUrls[pieceChar] || `/static/images/${piece.color === 'w' ? 'white' : 'black'}_${piece.type}.png`;
                    pieceImg.style.width = '100%';
                    pieceImg.style.height = '100%';
                    pieceImg.style.objectFit = 'contain';
                    square.appendChild(pieceImg);
                }
            }
        }
    }
}

// Flip view
function flipReplayView() {
    replayViewColor = replayViewColor === 'white' ? 'black' : 'white';
    updateSquareLabels();
    renderBoard(chess ? chess.fen() : 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
    updatePlayerNames();
}

// Update player names based on view
function updatePlayerNames() {
    if (!gameData) return;
    
    const whiteName = gameData.whitePlayer || 'White';
    const blackName = gameData.blackPlayer || 'Black';
    
    if (replayViewColor === 'white') {
        // White view: black above (opponent), white below (player)
        document.getElementById('opponent-name-board-text').textContent = blackName;
        document.getElementById('player-name-board-text').textContent = whiteName;
    } else {
        // Black view: white above (opponent), black below (player)
        document.getElementById('opponent-name-board-text').textContent = whiteName;
        document.getElementById('player-name-board-text').textContent = blackName;
    }
}

// Load game from localStorage
function loadGame(gameId) {
    currentGameId = gameId; // Store game ID for delete/download
    gameData = getGame(gameId);
    
    if (!gameData) {
        alert('Game not found.');
        return false;
    }
    
    // Initialize chess position
    chess = new Chess();
    currentMoveIndex = -1;
    replayViewColor = 'white'; // Start with white view
    
    // Update player names
    updatePlayerNames();
    
    // Render initial position
    renderBoard(chess.fen());
    updateMoveNotation();
    updateStatus();
    
    // Initialize Stockfish and analyze game
    initializeStockfish();
    
    return true;
}

// Delete the current game
function deleteCurrentGame() {
    if (!currentGameId) {
        alert('No game loaded.');
        return;
    }
    
    if (!confirm('Are you sure you want to delete this game? This action cannot be undone.')) {
        return;
    }
    
    if (deleteGame(currentGameId)) {
        alert('Game deleted successfully.');
        // Redirect back to lobby
        window.location.href = '/';
    } else {
        alert('Failed to delete game.');
    }
}

// Download PGN for the current game
function downloadCurrentGamePGN() {
    if (!currentGameId || !gameData) {
        alert('No game loaded.');
        return;
    }
    
    if (!gameData.pgn) {
        alert('PGN not available for this game.');
        return;
    }
    
    const blob = new Blob([gameData.pgn], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `chess_game_${gameData.timestamp}.pgn`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

// Go to specific move index
function goToMove(moveIndex) {
    if (!gameData || !gameData.moveHistory || gameData.moveHistory.length === 0) {
        // If no moves, just show starting position
        if (gameData) {
            chess = new Chess();
            renderBoard(chess.fen());
            updateStatus();
            updateControls();
        }
        return;
    }
    
    // Clamp move index
    moveIndex = Math.max(-1, Math.min(moveIndex, gameData.moveHistory.length - 1));
    currentMoveIndex = moveIndex;
    
    // Reset chess position
    chess = new Chess();
    
    // Play moves up to current index
    for (let i = 0; i <= moveIndex; i++) {
        const moveUci = gameData.moveHistory[i];
        if (moveUci && moveUci.length >= 4) {
            try {
                const from = moveUci.substring(0, 2);
                const to = moveUci.substring(2, 4);
                const promotion = moveUci.length > 4 ? moveUci.substring(4, 5) : undefined;
                
                chess.move({
                    from: from,
                    to: to,
                    promotion: promotion
                });
            } catch (e) {
                console.warn('Error playing move:', e);
            }
        }
    }
    
    // Render board
    renderBoard(chess.fen());
    updateMoveNotation();
    updateStatus();
    updateControls();
}

// Update move notation display with analysis
function updateMoveNotation() {
    const moveNotationDisplay = document.getElementById('move-notation-display');
    if (!moveNotationDisplay || !gameData) return;
    
    moveNotationDisplay.innerHTML = '';
    
    if (!gameData.moveSanHistory || gameData.moveSanHistory.length === 0) {
        return;
    }
    
    let moveNumber = 1;
    for (let i = 0; i < gameData.moveSanHistory.length; i++) {
        const san = gameData.moveSanHistory[i];
        if (!san) continue;
        
        const isWhite = i % 2 === 0;
        const isCurrentMove = i === currentMoveIndex;
        
        if (isWhite) {
            const numberNode = document.createTextNode(`${moveNumber}. `);
            moveNotationDisplay.appendChild(numberNode);
        } else if (i > 0) {
            const spaceNode = document.createTextNode(' ');
            moveNotationDisplay.appendChild(spaceNode);
        }
        
        // Get analysis data if available
        const analysis = moveAnalysis && moveAnalysis[i] ? moveAnalysis[i] : null;
        const annotation = analysis ? getMoveAnnotation(analysis.quality) : '';
        
        // Create move container
        const moveContainer = document.createElement('span');
        moveContainer.className = 'move-with-annotation';
        
        const moveSpan = document.createElement('span');
        moveSpan.className = 'move-text';
        moveSpan.textContent = san;
        
        if (isCurrentMove) {
            moveSpan.style.backgroundColor = '#ffeb3b';
            moveSpan.style.padding = '2px 4px';
            moveSpan.style.borderRadius = '2px';
        }
        
        moveContainer.appendChild(moveSpan);
        
        // Add annotation symbol if available
        if (annotation) {
            const annotationSpan = document.createElement('span');
            annotationSpan.className = `move-annotation annotation-${analysis.quality}`;
            annotationSpan.textContent = annotation;
            annotationSpan.style.marginLeft = '2px';
            moveContainer.appendChild(annotationSpan);
        }
        
        moveNotationDisplay.appendChild(moveContainer);
        
        if (!isWhite) {
            moveNumber++;
        }
    }
}

// Update game status
function updateStatus() {
    const statusText = document.getElementById('game-status-text');
    if (!statusText || !gameData) return;
    
    if (currentMoveIndex === -1) {
        statusText.textContent = 'Start position';
    } else if (currentMoveIndex === gameData.moveHistory.length - 1) {
        statusText.textContent = `Game Over: ${gameData.result} (${gameData.resultReason || 'Game completed'})`;
    } else {
        const moveNum = Math.floor(currentMoveIndex / 2) + 1;
        statusText.textContent = `Move ${moveNum}`;
    }
}

// Update control buttons state
function updateControls() {
    // Get all navigation buttons
    const rewindStartBtn = document.getElementById('rewind-start-btn');
    const rewindOneBtn = document.getElementById('rewind-one-btn');
    const playBtn = document.getElementById('play-btn');
    const pauseBtn = document.getElementById('pause-btn');
    const forwardOneBtn = document.getElementById('forward-one-btn');
    const forwardEndBtn = document.getElementById('forward-end-btn');
    
    // Check if game has no moves
    if (!gameData || !gameData.moveHistory || gameData.moveHistory.length === 0) {
        // Disable all navigation buttons
        if (rewindStartBtn) rewindStartBtn.disabled = true;
        if (rewindOneBtn) rewindOneBtn.disabled = true;
        if (playBtn) {
            playBtn.style.display = 'inline-block';
            playBtn.disabled = true;
        }
        if (pauseBtn) pauseBtn.style.display = 'none';
        if (forwardOneBtn) forwardOneBtn.disabled = true;
        if (forwardEndBtn) forwardEndBtn.disabled = true;
        return;
    }
    
    const maxMoveIndex = gameData.moveHistory.length - 1;
    const atStart = currentMoveIndex === -1;
    const atEnd = currentMoveIndex >= maxMoveIndex;
    
    // Update rewind buttons - disabled ONLY at start, enabled everywhere else (including at end)
    // When at the end, you should be able to rewind backwards
    if (rewindStartBtn) rewindStartBtn.disabled = atStart;
    if (rewindOneBtn) rewindOneBtn.disabled = atStart;
    
    // Update forward buttons - disabled at end, enabled otherwise
    if (forwardOneBtn) forwardOneBtn.disabled = atEnd;
    if (forwardEndBtn) forwardEndBtn.disabled = atEnd;
    
    // Update play/pause buttons
    if (atEnd) {
        // Ensure we're paused when at the end (but don't call pause() here to avoid recursion)
        if (isPlaying) {
            isPlaying = false;
            if (playInterval) {
                clearInterval(playInterval);
                playInterval = null;
            }
        }
        if (playBtn) {
            playBtn.style.display = 'inline-block';
            playBtn.disabled = true;
        }
        if (pauseBtn) pauseBtn.style.display = 'none';
    } else {
        if (isPlaying) {
            if (playBtn) playBtn.style.display = 'none';
            if (pauseBtn) pauseBtn.style.display = 'inline-block';
            if (pauseBtn) pauseBtn.disabled = false;
        } else {
            if (playBtn) {
                playBtn.style.display = 'inline-block';
                playBtn.disabled = false;
            }
            if (pauseBtn) pauseBtn.style.display = 'none';
        }
    }
}

// Replay controls
function rewindToStart() {
    if (!gameData) return;
    pause();
    goToMove(-1);
}

function rewindOneMove() {
    if (!gameData) return;
    pause();
    goToMove(currentMoveIndex - 1);
}

function play() {
    if (!gameData || !gameData.moveHistory || gameData.moveHistory.length === 0) {
        return;
    }
    
    if (currentMoveIndex >= gameData.moveHistory.length - 1) {
        return;
    }
    
    isPlaying = true;
    updateControls();
    
    playInterval = setInterval(() => {
        if (!gameData || !gameData.moveHistory) {
            pause();
            return;
        }
        if (currentMoveIndex >= gameData.moveHistory.length - 1) {
            pause();
            return;
        }
        goToMove(currentMoveIndex + 1);
    }, 1000);
}

function pause() {
    isPlaying = false;
    if (playInterval) {
        clearInterval(playInterval);
        playInterval = null;
    }
    updateControls();
}

function fastForwardOneMove() {
    if (!gameData) return;
    pause();
    goToMove(currentMoveIndex + 1);
}

function fastForwardToEnd() {
    if (!gameData || !gameData.moveHistory || gameData.moveHistory.length === 0) {
        return;
    }
    pause();
    goToMove(gameData.moveHistory.length - 1);
}

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
    await loadPieceImages();
    createChessboard();
    
    // Get gameId from URL
    const urlParams = new URLSearchParams(window.location.search);
    const gameId = urlParams.get('gameId');
    
    if (!gameId) {
        alert('No game ID provided.');
        return;
    }
    
    if (!loadGame(gameId)) {
        return;
    }
    
    // Setup replay controls - add error handling
    const rewindStartBtn = document.getElementById('rewind-start-btn');
    const rewindOneBtn = document.getElementById('rewind-one-btn');
    const playBtn = document.getElementById('play-btn');
    const pauseBtn = document.getElementById('pause-btn');
    const forwardOneBtn = document.getElementById('forward-one-btn');
    const forwardEndBtn = document.getElementById('forward-end-btn');
    const flipViewBtn = document.getElementById('flip-view-btn');
    
    if (rewindStartBtn) {
        rewindStartBtn.addEventListener('click', rewindToStart);
    } else {
        console.error('[Replay] rewind-start-btn not found');
    }
    
    if (rewindOneBtn) {
        rewindOneBtn.addEventListener('click', rewindOneMove);
    } else {
        console.error('[Replay] rewind-one-btn not found');
    }
    
    if (playBtn) {
        playBtn.addEventListener('click', play);
    } else {
        console.error('[Replay] play-btn not found');
    }
    
    if (pauseBtn) {
        pauseBtn.addEventListener('click', pause);
    } else {
        console.error('[Replay] pause-btn not found');
    }
    
    if (forwardOneBtn) {
        forwardOneBtn.addEventListener('click', fastForwardOneMove);
    } else {
        console.error('[Replay] forward-one-btn not found');
    }
    
    if (forwardEndBtn) {
        forwardEndBtn.addEventListener('click', fastForwardToEnd);
    } else {
        console.error('[Replay] forward-end-btn not found');
    }
    
    if (flipViewBtn) {
        flipViewBtn.addEventListener('click', flipReplayView);
    } else {
        console.error('[Replay] flip-view-btn not found');
    }
    
    // Setup delete and download buttons
    const deleteGameBtn = document.getElementById('delete-game-btn');
    const downloadPgnBtn = document.getElementById('download-pgn-btn');
    
    if (deleteGameBtn) {
        deleteGameBtn.addEventListener('click', deleteCurrentGame);
    } else {
        console.error('[Replay] delete-game-btn not found');
    }
    
    if (downloadPgnBtn) {
        downloadPgnBtn.addEventListener('click', downloadCurrentGamePGN);
    } else {
        console.error('[Replay] download-pgn-btn not found');
    }
    
    updateControls();
});
