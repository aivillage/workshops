import { state } from './state.js';

/**
 * Play a sound effect
 * @param {string} soundFile - Name of the sound file (e.g., 'move.mp3')
 */
export function playSound(soundFile) {
    // Only play sounds when in an active game (not in lobby)
    const lobby = document.getElementById('lobby');
    const isInLobby = lobby && lobby.style.display !== 'none';
    
    if (!state.currentGame || !state.gameStarted || isInLobby) {
        return;
    }
    
    try {
        const audio = new Audio(`/static/sounds/${soundFile}`);
        audio.volume = 0.5; // Set volume to 50%
        audio.play().catch(err => {
            // Ignore errors (e.g., user hasn't interacted with page yet)
            console.log('Sound play prevented:', err);
        });
    } catch (error) {
        console.error('Error playing sound:', error);
    }
}

