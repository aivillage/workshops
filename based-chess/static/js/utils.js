import { state } from './state.js';

/**
 * Format time in seconds to MM:SS format
 * @param {number} seconds - Time in seconds
 * @returns {string} Formatted time string
 */
export function formatTime(seconds) {
    if (seconds === null || seconds === undefined) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
}

/**
 * Load piece images as base64 from server
 */
export async function loadPieceImages() {
    try {
        const response = await fetch('/piece-images');
        if (response.ok) {
            const data = await response.json();
            state.pieceImageUrls = data;
            
            // Preload images to ensure they're cached
            Object.values(state.pieceImageUrls).forEach(dataUri => {
                const img = new Image();
                img.src = dataUri;
            });
        } else {
            console.warn('Failed to load piece images, falling back to static URLs');
            // Fallback to static URLs if base64 loading fails
            state.pieceImageUrls = {
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
        // Fallback to static URLs
        state.pieceImageUrls = {
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

/**
 * Get square name from row and column
 * @param {number} row - Row index (0-7)
 * @param {number} col - Column index (0-7)
 * @returns {string} Square name (e.g., 'e4')
 */
export function getSquareName(row, col) {
    // Convert row/col to square name
    // Invert row (chessboard is displayed with rank 8 at top)
    let displayRow = 7 - row;
    let displayCol = col;
    
    // For spectators, use their view color; for players, use their actual color
    const viewColor = state.isSpectator ? state.spectatorViewColor : state.myColor;
    
    // Invert for black view (if color assigned, otherwise use normal view)
    if (viewColor === 'black') {
        displayRow = 7 - displayRow;
        displayCol = 7 - displayCol;
    }
    
    const file = String.fromCharCode('a'.charCodeAt(0) + displayCol);
    const rank = displayRow + 1;
    return file + rank;
}

/**
 * Get square element by name
 * @param {string} squareName - Square name (e.g., 'e4')
 * @returns {HTMLElement|null} Square element or null
 */
export function getSquareElement(squareName) {
    // Convert square name (e.g., 'e4') to row/col
    const file = squareName[0].charCodeAt(0) - 'a'.charCodeAt(0);
    const rank = parseInt(squareName[1]) - 1;
    
    // For spectators, use their view color; for players, use their actual color
    const viewColor = state.isSpectator ? state.spectatorViewColor : state.myColor;
    
    // Invert for black view (if color assigned, otherwise show normal view)
    let row, col;
    if (viewColor === 'black') {
        row = 7 - rank;
        col = 7 - file;
    } else {
        row = rank;
        col = file;
    }
    
    // Invert row (chessboard is displayed with rank 8 at top)
    row = 7 - row;
    
    return document.querySelector(`.square[data-row="${row}"][data-col="${col}"]`);
}

