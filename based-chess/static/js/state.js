// Global application state
export const state = {
    // WebSocket connection
    ws: null,
    playerId: null,
    playerName: null,
    
    // Game state
    currentGame: null,
    myColor: null,
    selectedSquare: null,
    validMoves: [],
    boardState: {},
    gameStarted: false,
    iAmReady: false,
    countdownTimer: null,
    
    // Promotion
    promotionChoices: ['q', 'n', 'r', 'b'], // queen, knight, rook, bishop
    promotionIndex: 0,
    
    // Drag and drop
    draggedPiece: null,
    dragStartSquare: null,
    
    // Lobby
    gamesListData: {},
    allGames: [],
    currentSortColumn: null,
    sortDirection: 'asc',
    
    // Game state tracking
    gameEndSoundPlayed: false,
    previousMoveCount: 0,
    currentGameState: null,
    
    // Rematch
    rematchRequested: false,
    rematchAccepted: false,
    rematchButtonClickCount: 0,
    rematchButtonOriginalText: '',
    rematchButtonTimeout: null,
    rematchTimeoutTimer: null,
    
    // Spectator
    isSpectator: false,
    spectatorViewColor: 'white',
    
    // Timer
    whiteTimeRemaining: null,
    blackTimeRemaining: null,
    lastTimeUpdate: null,
    timerInterval: null,
    
    // Bot state
    playAgainstBot: false,
    botColor: null,
    botThinking: false,
    botWs: null,
    botPlayerId: null,
    botRoomId: null,
    botPassword: null,
    stockfishEngine: null,
    stockfishCallback: null,
    botLevel: 10,
    lastProcessedTurn: null,
    
    // Piece images
    pieceImageUrls: {},
    
    // Draw
    drawButtonClickCount: 0,
    drawButtonOriginalText: '',
    hasDrawOffer: false,
    previousDrawOfferedBy: null,
    drawButtonTimeout: null,
    
    // Resign
    resignButtonClickCount: 0,
    resignButtonOriginalText: '',
    resignButtonTimeout: null
};

