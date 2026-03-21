import { state } from './state.js';

// These will be injected from app.js
let messageHandlers = {};

/**
 * Initialize WebSocket module with message handlers
 */
export function initWebSocket(handlers) {
    messageHandlers = handlers;
}

/**
 * Connect to WebSocket server
 */
export function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    state.ws = new WebSocket(wsUrl);
    
    state.ws.onopen = () => {
        // Connection established
    };
    
    state.ws.onmessage = (event) => {
        const message = JSON.parse(event.data);
        handleMessage(message);
    };
    
    state.ws.onclose = () => {
        if (messageHandlers.updateGameStatus) {
            messageHandlers.updateGameStatus('Disconnected. Reconnecting...');
        }
        // Disable start button when disconnected
        const startBtn = document.getElementById('start-game-btn');
        if (startBtn) {
            startBtn.disabled = true;
            startBtn.textContent = 'Connecting...';
        }
        // Reconnect after 2 seconds
        setTimeout(connectWebSocket, 2000);
    };
    
    state.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        if (messageHandlers.updateGameStatus) {
            messageHandlers.updateGameStatus('Connection error. Reconnecting...');
        }
    };
}

/**
 * Handle incoming WebSocket messages
 */
function handleMessage(message) {
    if (!messageHandlers.handleMessage) {
        console.error('Message handler not initialized');
        return;
    }
    messageHandlers.handleMessage(message);
}

/**
 * Send a message through WebSocket
 */
export function sendMessage(message) {
    if (state.ws && state.ws.readyState === WebSocket.OPEN) {
        state.ws.send(JSON.stringify(message));
    } else {
        console.error('WebSocket not connected');
    }
}

