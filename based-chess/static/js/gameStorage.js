/**
 * Game Storage Module
 * Manages localStorage for saving completed chess games (max 100 games)
 */

const STORAGE_KEY = 'chess_games';
const MAX_GAMES = 100;

/**
 * Save a completed game to localStorage
 * @param {Object} gameData - Game data to save
 * @returns {boolean} Success status
 */
export function saveGame(gameData) {
    try {
        const games = getAllGames();
        
        // Check if a game with the same gameId already exists
        const existingIndex = games.findIndex(game => game.gameId === gameData.gameId);
        
        if (existingIndex !== -1) {
            // Update existing game (e.g., when analysis completes after initial save)
            games[existingIndex] = gameData;
        } else {
            // Add new game
            games.push(gameData);
            
            // Remove oldest games if we're at the limit
            if (games.length > MAX_GAMES) {
                // Sort by timestamp and remove oldest
                games.sort((a, b) => a.timestamp - b.timestamp);
                const gamesToKeep = games.slice(-MAX_GAMES);
                localStorage.setItem(STORAGE_KEY, JSON.stringify(gamesToKeep));
                return true;
            }
        }
        
        // Save back to localStorage
        localStorage.setItem(STORAGE_KEY, JSON.stringify(games));
        return true;
    } catch (error) {
        console.error('Error saving game:', error);
        return false;
    }
}

/**
 * Get all saved games from localStorage
 * @returns {Array} Array of game data objects
 */
export function getAllGames() {
    try {
        const gamesJson = localStorage.getItem(STORAGE_KEY);
        if (!gamesJson) {
            return [];
        }
        const games = JSON.parse(gamesJson);
        // Sort by timestamp descending (newest first)
        return games.sort((a, b) => b.timestamp - a.timestamp);
    } catch (error) {
        console.error('Error loading games:', error);
        return [];
    }
}

/**
 * Get a specific game by gameId
 * @param {string} gameId - Game ID to retrieve
 * @returns {Object|null} Game data or null if not found
 */
export function getGame(gameId) {
    try {
        const games = getAllGames();
        return games.find(game => game.gameId === gameId) || null;
    } catch (error) {
        console.error('Error getting game:', error);
        return null;
    }
}

/**
 * Delete a game by gameId
 * @param {string} gameId - Game ID to delete
 * @returns {boolean} Success status
 */
export function deleteGame(gameId) {
    try {
        const games = getAllGames();
        const filteredGames = games.filter(game => game.gameId !== gameId);
        localStorage.setItem(STORAGE_KEY, JSON.stringify(filteredGames));
        return true;
    } catch (error) {
        console.error('Error deleting game:', error);
        return false;
    }
}

/**
 * Clear all saved games
 * @returns {boolean} Success status
 */
export function clearAllGames() {
    try {
        localStorage.removeItem(STORAGE_KEY);
        return true;
    } catch (error) {
        console.error('Error clearing games:', error);
        return false;
    }
}

