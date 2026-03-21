import { state } from './state.js';
import { getSquareName, getSquareElement } from './utils.js';

// These functions will be imported from other modules
let updateGameStatus = null;
let renderBoard = null;
let currentPromotionLetter = null;

/**
 * Initialize chessboard module with dependencies
 */
export function initChessboard(deps) {
    updateGameStatus = deps.updateGameStatus;
    renderBoard = deps.renderBoard;
    currentPromotionLetter = deps.currentPromotionLetter;
}

/**
 * Create the chessboard DOM structure
 */
export function createChessboard() {
    const board = document.getElementById('chessboard');
    board.innerHTML = '';
    
    // Create 64 squares
    for (let row = 0; row < 8; row++) {
        for (let col = 0; col < 8; col++) {
            const square = document.createElement('div');
            square.className = 'square';
            square.classList.add((row + col) % 2 === 0 ? 'light' : 'dark');
            square.dataset.row = row;
            square.dataset.col = col;
            // Add click handler - spectators will be blocked in handleSquareClick
            square.addEventListener('click', () => handleSquareClick(row, col));
            
            // Add square label element
            const label = document.createElement('span');
            label.className = 'square-label';
            square.appendChild(label);
            
            board.appendChild(square);
        }
    }
    
    // Setup drag and drop
    setupDragAndDrop();
    // Update square labels
    updateSquareLabels();
}

/**
 * Update square labels to match replay style: file (a-h) on bottom rank, rank (1-8) on left file.
 * Respects view (player color or spectator flip).
 */
export function updateSquareLabels() {
    const squares = document.querySelectorAll('.square');
    const viewColor = state.isSpectator ? state.spectatorViewColor : (state.myColor || 'white');

    squares.forEach(square => {
        const row = parseInt(square.dataset.row);
        const col = parseInt(square.dataset.col);

        // Display coordinates: from current view, bottom = rank 1, left = file a
        let displayRow, displayCol;
        if (viewColor === 'white') {
            displayRow = row;   // row 0 = top (rank 8), row 7 = bottom (rank 1)
            displayCol = col;   // col 0 = left (a), col 7 = right (h)
        } else {
            displayRow = 7 - row;
            displayCol = 7 - col;
        }

        const rank = 8 - displayRow; // displayRow 0 = rank 8, displayRow 7 = rank 1
        const file = String.fromCharCode('a'.charCodeAt(0) + displayCol);

        let label = square.querySelector('.square-label');
        if (!label) {
            label = document.createElement('span');
            label.className = 'square-label';
            square.appendChild(label);
        }

        // File labels (a-h) on bottom rank; rank labels (1-8) on left file
        if (displayRow === 7) {
            label.textContent = file;
            label.style.display = 'block';
        } else if (displayCol === 0) {
            label.textContent = String(rank);
            label.style.display = 'block';
        } else {
            label.textContent = '';
            label.style.display = 'none';
        }
    });
}

/**
 * Handle square click
 */
function handleSquareClick(row, col) {
    if (!state.currentGame) return;
    
    // Spectators can't make moves
    if (state.isSpectator) {
        return;
    }
    
    // Don't allow moves if color not assigned yet
    if (!state.myColor) {
        if (updateGameStatus) {
            updateGameStatus('Colors not assigned yet. Please wait for both players to be ready.');
        }
        return;
    }
    
    // Don't allow moves if game hasn't started
    if (!state.gameStarted) {
        return;
    }
    
    const squareName = getSquareName(row, col);
    
    // If no square selected, select this square if it has a piece of player's color
    if (!state.selectedSquare) {
        const piece = state.boardState[squareName];
        if (piece && piece.color === state.myColor) {
            state.selectedSquare = squareName;
            state.validMoves = [];
            // Request legal moves from server
            if (state.ws && state.ws.readyState === WebSocket.OPEN) {
                state.ws.send(JSON.stringify({
                    type: 'get_legal_moves',
                    square: squareName
                }));
            }
            if (renderBoard) renderBoard();
        }
    } else {
        // Check if clicking on a valid move square
        if (state.validMoves.includes(squareName)) {
            // Try to make a move
            let move = state.selectedSquare + squareName;
            
            // Check if this is a pawn promotion
            const fromPiece = state.boardState[state.selectedSquare];
            if (fromPiece) {
                const pieceType = fromPiece.type.toUpperCase();
                const isPawn = pieceType === 'P';
                const targetRank = parseInt(squareName[1]);
                
                // Pawn promotion: white pawn to rank 8, or black pawn to rank 1
                if (isPawn && ((state.myColor === 'white' && targetRank === 8) || (state.myColor === 'black' && targetRank === 1))) {
                    if (currentPromotionLetter) {
                        move = move + currentPromotionLetter();
                    }
                }
            }
            
            state.ws.send(JSON.stringify({
                type: 'make_move',
                move: move
            }));
            state.selectedSquare = null;
            state.validMoves = [];
            if (renderBoard) renderBoard();
        } else {
            // Clicked on a different square - either select new piece or deselect
            const piece = state.boardState[squareName];
            if (piece && piece.color === state.myColor) {
                // Select new piece
                state.selectedSquare = squareName;
                state.validMoves = [];
                if (state.ws && state.ws.readyState === WebSocket.OPEN) {
                    state.ws.send(JSON.stringify({
                        type: 'get_legal_moves',
                        square: squareName
                    }));
                }
                if (renderBoard) renderBoard();
            } else {
                // Deselect
                state.selectedSquare = null;
                state.validMoves = [];
                if (renderBoard) renderBoard();
            }
        }
    }
}

/**
 * Render the chessboard with pieces
 */
export function renderBoard() {
    const squares = document.querySelectorAll('.square');
    const chessboard = document.getElementById('chessboard');
    
    // Update square labels first (in case color changed)
    updateSquareLabels();
    
    // Clear all pieces and highlights, but keep square labels
    squares.forEach(square => {
        // Keep square label when clearing
        const label = square.querySelector('.square-label');
        const labelText = label ? label.textContent : '';
        
        // Remove everything except label
        square.innerHTML = '';
        if (label) {
            label.textContent = labelText;
            square.appendChild(label);
        }
        
        square.classList.remove('selected', 'valid-move');
        // Spectators should not have pointer cursor on squares
        if (state.isSpectator) {
            square.style.cursor = 'default';
            square.style.pointerEvents = 'none';
        } else {
            square.style.cursor = 'pointer';
            square.style.pointerEvents = '';
        }
    });
    
    if (chessboard) {
        chessboard.style.cursor = '';
    }
    
    // Render pieces
    if (!state.boardState || typeof state.boardState !== 'object') {
        console.warn('Invalid boardState:', state.boardState);
        state.boardState = {};
    }
    Object.keys(state.boardState).forEach(squareName => {
        const piece = state.boardState[squareName];
        if (piece) {
            const square = getSquareElement(squareName);
            if (square) {
                // Preserve square label
                const label = square.querySelector('.square-label');
                
                const typeKey = (piece.type || '').toString();
                const imageSrc = state.pieceImageUrls[typeKey]
                    || state.pieceImageUrls[typeKey.toUpperCase?.() ? typeKey.toUpperCase() : typeKey]
                    || state.pieceImageUrls[typeKey.toLowerCase?.() ? typeKey.toLowerCase() : typeKey];
                if (imageSrc) {
                    const img = document.createElement('img');
                    img.src = imageSrc;
                    img.alt = typeKey;
                    img.className = 'piece piece-img';
                    img.onerror = () => { 
                        if (label) {
                            square.innerHTML = '';
                            square.appendChild(label);
                        }
                    };
                    
                    // Clear and add both piece and label
                    square.innerHTML = '';
                    square.appendChild(img);
                    if (label) {
                        square.appendChild(label);
                    }
                    
                    // Make square draggable if it's the player's piece (not for spectators)
                    if (!state.isSpectator && piece.color === state.myColor && state.gameStarted) {
                        square.draggable = true;
                        square.dataset.pieceSquare = squareName;
                    } else {
                        square.draggable = false;
                    }
                } else {
                    // No known source for this piece type; restore label
                    square.innerHTML = '';
                    if (label) {
                        square.appendChild(label);
                    }
                }
            }
        }
    });
    
    // Highlight selected square
    if (state.selectedSquare) {
        const square = getSquareElement(state.selectedSquare);
        if (square) {
            square.classList.add('selected');
        }
    }
    
    // Highlight valid moves
    state.validMoves.forEach(squareName => {
        const square = getSquareElement(squareName);
        if (square) {
            square.classList.add('valid-move');
        }
    });
}

/**
 * Setup drag and drop handlers
 */
function setupDragAndDrop() {
    const squares = document.querySelectorAll('.square');
    squares.forEach(square => {
        // Remove old listeners if any
        square.removeEventListener('dragover', handleDragOver);
        square.removeEventListener('drop', handleDrop);
        square.removeEventListener('dragenter', handleDragEnter);
        square.removeEventListener('dragleave', handleDragLeave);
        square.removeEventListener('dragstart', handleSquareDragStart);
        square.removeEventListener('dragend', handleSquareDragEnd);
        
        // Add new listeners
        square.addEventListener('dragover', handleDragOver);
        square.addEventListener('drop', handleDrop);
        square.addEventListener('dragenter', handleDragEnter);
        square.addEventListener('dragleave', handleDragLeave);
        
        // Add drag start/end to draggable squares
        if (square.draggable) {
            square.addEventListener('dragstart', handleSquareDragStart);
            square.addEventListener('dragend', handleSquareDragEnd);
        }
    });
}

/**
 * Handle drag start
 */
function handleDragStart(e, squareName) {
    if (!state.currentGame || !state.gameStarted || !state.myColor || state.isSpectator) {
        e.preventDefault();
        if (state.isSpectator) {
            return;
        }
        if (!state.myColor && updateGameStatus) {
            updateGameStatus('Colors not assigned yet. Please wait for both players to be ready.');
        }
        return;
    }
    
    const piece = state.boardState[squareName];
    if (!piece || piece.color !== state.myColor) {
        e.preventDefault();
        return;
    }
    
    state.draggedPiece = piece;
    state.dragStartSquare = squareName;
    state.selectedSquare = squareName;
    state.validMoves = [];
    
    // Request legal moves
    if (state.ws && state.ws.readyState === WebSocket.OPEN) {
        state.ws.send(JSON.stringify({
            type: 'get_legal_moves',
            square: squareName
        }));
    }
    
    // Set drag image
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', squareName);
    
    // Make piece semi-transparent while dragging
    const square = e.currentTarget;
    if (square) {
        square.style.opacity = '0.5';
    }
    
    if (renderBoard) renderBoard();
}

/**
 * Handle drag end
 */
function handleDragEnd(e) {
    e.target.style.opacity = '1';
    state.draggedPiece = null;
    state.dragStartSquare = null;
}

/**
 * Handle square drag start
 */
function handleSquareDragStart(e) {
    const squareName = e.currentTarget.dataset.pieceSquare;
    if (!squareName) return;
    handleDragStart(e, squareName);
}

/**
 * Handle square drag end
 */
function handleSquareDragEnd(e) {
    e.currentTarget.style.opacity = '1';
    handleDragEnd(e);
}

/**
 * Handle drag over
 */
function handleDragOver(e) {
    if (state.isSpectator) {
        return;
    }
    
    if (state.draggedPiece && state.dragStartSquare) {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
    }
}

/**
 * Handle drag enter
 */
function handleDragEnter(e) {
    if (state.isSpectator) {
        return;
    }
    
    if (state.draggedPiece && state.dragStartSquare) {
        e.preventDefault();
        const row = parseInt(e.currentTarget.dataset.row);
        const col = parseInt(e.currentTarget.dataset.col);
        const squareName = getSquareName(row, col);
        
        if (state.validMoves.includes(squareName)) {
            e.currentTarget.classList.add('drag-over-valid');
        }
    }
}

/**
 * Handle drag leave
 */
function handleDragLeave(e) {
    e.currentTarget.classList.remove('drag-over-valid');
}

/**
 * Handle drop
 */
function handleDrop(e) {
    e.preventDefault();
    e.currentTarget.classList.remove('drag-over-valid');
    
    if (state.isSpectator) {
        return;
    }
    
    if (!state.draggedPiece || !state.dragStartSquare) return;
    
    const row = parseInt(e.currentTarget.dataset.row);
    const col = parseInt(e.currentTarget.dataset.col);
    const squareName = getSquareName(row, col);
    
    // Check if this is a valid move
    if (state.validMoves.includes(squareName)) {
        let move = state.dragStartSquare + squareName;
        
        // Check if this is a pawn promotion
        const fromPiece = state.boardState[state.dragStartSquare];
        if (fromPiece) {
            const pieceType = fromPiece.type.toUpperCase();
            const isPawn = pieceType === 'P';
            const targetRank = parseInt(squareName[1]);
            
            // Pawn promotion: white pawn to rank 8, or black pawn to rank 1
            if (isPawn && ((state.myColor === 'white' && targetRank === 8) || (state.myColor === 'black' && targetRank === 1))) {
                if (currentPromotionLetter) {
                    move = move + currentPromotionLetter();
                }
            }
        }
        
        state.ws.send(JSON.stringify({
            type: 'make_move',
            move: move
        }));
        
        state.selectedSquare = null;
        state.validMoves = [];
        state.draggedPiece = null;
        state.dragStartSquare = null;
        if (renderBoard) renderBoard();
    }
}

// Export setupDragAndDrop for use after board render
export { setupDragAndDrop };

