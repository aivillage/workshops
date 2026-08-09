import os
import requests
from flask import Flask, render_template_string, request, jsonify

app = Flask(__name__)

SERVICE_URL = os.environ.get("SERVICE_URL", "http://llm-comparison-service:5000")

HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>LLM Parallel Exploration & Comparison Studio</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=Outfit:wght@400;600;700;800&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-dark: #0f172a;
            --bg-card: rgba(30, 41, 59, 0.7);
            --border-card: rgba(255, 255, 255, 0.1);
            --text-main: #f8fafc;
            --text-muted: #94a3b8;
            --accent-a: #6366f1;
            --accent-a-glow: rgba(99, 102, 241, 0.25);
            --accent-b: #ec4899;
            --accent-b-glow: rgba(236, 72, 153, 0.25);
            --accent-emerald: #10b981;
            --accent-amber: #f59e0b;
            --accent-red: #ef4444;
            --accent-blue: #3b82f6;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            font-family: 'Inter', sans-serif;
            background: var(--bg-dark);
            color: var(--text-main);
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            overflow-x: hidden;
        }

        /* Top Header */
        header {
            background: rgba(15, 23, 42, 0.95);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--border-card);
            padding: 0.85rem 2rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: sticky;
            top: 0;
            z-index: 100;
        }

        .brand {
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }

        .brand-icon {
            font-size: 1.6rem;
            background: linear-gradient(135deg, var(--accent-a), var(--accent-b));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .brand-title {
            font-family: 'Outfit', sans-serif;
            font-size: 1.3rem;
            font-weight: 700;
            letter-spacing: -0.02em;
        }

        .header-actions {
            display: flex;
            gap: 0.75rem;
            align-items: center;
        }

        .btn-header {
            background: rgba(255, 255, 255, 0.06);
            color: var(--text-main);
            border: 1px solid var(--border-card);
            padding: 0.45rem 1rem;
            border-radius: 8px;
            font-weight: 600;
            font-size: 0.85rem;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            gap: 0.4rem;
        }

        .btn-header:hover {
            background: rgba(255, 255, 255, 0.12);
            transform: translateY(-1px);
        }

        .btn-reset {
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
            border: 1px solid rgba(239, 68, 68, 0.3);
        }

        .btn-reset:hover {
            background: rgba(239, 68, 68, 0.3);
            border-color: #ef4444;
        }

        /* Collapsible Controls Section */
        .controls-bar {
            background: rgba(30, 41, 59, 0.4);
            border-bottom: 1px solid var(--border-card);
            padding: 1rem 2rem;
            transition: all 0.3s ease;
        }

        .controls-bar.collapsed .model-grid {
            display: none;
        }

        .controls-summary {
            display: none;
            align-items: center;
            justify-content: space-between;
            font-size: 0.88rem;
            color: var(--text-muted);
        }

        .controls-bar.collapsed .controls-summary {
            display: flex;
        }

        .model-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1.5rem;
        }

        .model-selector-card {
            background: var(--bg-card);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 0.85rem 1.1rem;
            transition: border-color 0.3s ease;
        }

        .model-selector-card.model-a { border-left: 4px solid var(--accent-a); }
        .model-selector-card.model-b { border-left: 4px solid var(--accent-b); }

        .selector-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }

        .selector-label {
            font-family: 'Outfit', sans-serif;
            font-weight: 700;
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        .model-a .selector-label { color: var(--accent-a); }
        .model-b .selector-label { color: var(--accent-b); }

        select, input[type="text"] {
            width: 100%;
            background: rgba(15, 23, 42, 0.8);
            color: var(--text-main);
            border: 1px solid var(--border-card);
            border-radius: 8px;
            padding: 0.55rem 0.75rem;
            font-family: 'Inter', sans-serif;
            font-size: 0.88rem;
            outline: none;
            transition: all 0.2s ease;
        }

        select:focus, input[type="text"]:focus {
            border-color: var(--accent-a);
            box-shadow: 0 0 0 3px var(--accent-a-glow);
        }

        select:disabled, input:disabled {
            opacity: 0.5;
            cursor: not-allowed;
            background: rgba(15, 23, 42, 0.4);
            border-color: rgba(255, 255, 255, 0.05);
        }

        .model-description {
            font-size: 0.78rem;
            color: var(--text-muted);
            margin-top: 0.4rem;
            line-height: 1.3;
        }

        .custom-model-toggle {
            font-size: 0.75rem;
            color: var(--text-muted);
            cursor: pointer;
            text-decoration: underline;
            margin-top: 0.3rem;
            display: inline-block;
        }

        /* Collapsible System Prompt */
        .system-prompt-card {
            background: var(--bg-card);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 0.85rem 1.1rem;
            margin-bottom: 1.5rem;
            transition: all 0.3s ease;
        }

        .system-prompt-header {
            font-size: 0.82rem;
            font-weight: 600;
            color: var(--text-muted);
            margin-bottom: 0.4rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
            cursor: pointer;
        }

        .system-prompt-body {
            transition: all 0.3s ease;
        }

        .system-prompt-card.collapsed .system-prompt-body {
            display: none;
        }

        /* Main Dual Stream Layout */
        main {
            flex: 1;
            padding: 1.25rem 2rem 140px;
            max-width: 1600px;
            width: 100%;
            margin: 0 auto;
        }

        .turns-container {
            display: flex;
            flex-direction: column;
            gap: 2rem;
        }

        .turn-block {
            background: rgba(15, 23, 42, 0.4);
            border: 1px solid var(--border-card);
            border-radius: 16px;
            padding: 1.25rem;
        }

        .turn-prompt-badge {
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            background: rgba(255, 255, 255, 0.08);
            border: 1px solid rgba(255, 255, 255, 0.15);
            padding: 0.4rem 0.9rem;
            border-radius: 20px;
            font-size: 0.88rem;
            font-weight: 600;
            margin-bottom: 1.25rem;
        }

        .dual-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1.5rem;
        }

        .response-card {
            background: var(--bg-card);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 1.25rem;
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            position: relative;
            transition: all 0.3s ease;
        }

        .response-card.card-a { border-top: 3px solid var(--accent-a); }
        .response-card.card-b { border-top: 3px solid var(--accent-b); }

        .response-card.selected-context {
            box-shadow: 0 0 0 2px var(--accent-emerald), 0 8px 24px rgba(16, 185, 129, 0.15);
            background: rgba(16, 185, 129, 0.05);
        }

        .response-card.card-errored {
            border-top-color: var(--accent-red);
            background: rgba(239, 68, 68, 0.05);
        }

        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1rem;
            padding-bottom: 0.75rem;
            border-bottom: 1px solid rgba(255, 255, 255, 0.08);
        }

        .model-badge {
            font-family: 'Outfit', sans-serif;
            font-weight: 700;
            font-size: 0.88rem;
        }

        .card-a .model-badge { color: var(--accent-a); }
        .card-b .model-badge { color: var(--accent-b); }
        .card-errored .model-badge { color: var(--accent-red); }

        .response-body {
            font-size: 0.95rem;
            line-height: 1.6;
            white-space: pre-wrap;
            color: #e2e8f0;
            flex: 1;
            margin-bottom: 1.25rem;
        }

        /* Response Metadata Pills */
        .meta-pills {
            display: flex;
            flex-wrap: wrap;
            gap: 0.5rem;
            margin-bottom: 1rem;
            padding-top: 0.75rem;
            border-top: 1px solid rgba(255, 255, 255, 0.08);
        }

        .pill {
            font-size: 0.75rem;
            font-weight: 600;
            padding: 0.25rem 0.6rem;
            border-radius: 6px;
            background: rgba(255, 255, 255, 0.06);
            color: var(--text-muted);
            border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .pill.latency { color: var(--accent-amber); border-color: rgba(245, 158, 11, 0.3); }
        .pill.tokens { color: var(--accent-emerald); border-color: rgba(16, 185, 129, 0.3); }

        .pill-clickable {
            cursor: pointer;
            transition: all 0.2s ease;
            background: rgba(16, 185, 129, 0.12);
        }

        .pill-clickable:hover {
            background: rgba(16, 185, 129, 0.3);
            border-color: var(--accent-emerald);
            color: #fff;
            transform: translateY(-1px);
        }

        /* Card Actions */
        .card-actions {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }

        .context-select-btn {
            width: 100%;
            background: rgba(255, 255, 255, 0.05);
            color: var(--text-muted);
            border: 1px solid var(--border-card);
            padding: 0.6rem;
            border-radius: 8px;
            font-size: 0.85rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            justify-content: center;
            align-items: center;
            gap: 0.5rem;
        }

        .context-select-btn:hover:not(:disabled) {
            background: rgba(255, 255, 255, 0.12);
            color: var(--text-main);
        }

        .context-select-btn:disabled {
            opacity: 0.4;
            cursor: not-allowed;
            background: rgba(255, 255, 255, 0.02);
            border-color: rgba(255, 255, 255, 0.05);
        }

        .selected-context .context-select-btn {
            background: var(--accent-emerald);
            color: #000;
            border-color: var(--accent-emerald);
            font-weight: 700;
        }

        .btn-retry-model {
            width: 100%;
            background: rgba(59, 130, 246, 0.15);
            color: #60a5fa;
            border: 1px solid rgba(59, 130, 246, 0.3);
            padding: 0.55rem;
            border-radius: 8px;
            font-size: 0.85rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            justify-content: center;
            align-items: center;
            gap: 0.4rem;
        }

        .btn-retry-model:hover {
            background: rgba(59, 130, 246, 0.3);
            border-color: var(--accent-blue);
        }

        /* Fixed Bottom Prompt Input Bar */
        .input-bar-container {
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            background: rgba(15, 23, 42, 0.95);
            backdrop-filter: blur(16px);
            border-top: 1px solid var(--border-card);
            padding: 1.25rem 2rem;
            z-index: 100;
        }

        .input-bar {
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            gap: 1rem;
            align-items: flex-end;
        }

        textarea {
            flex: 1;
            background: rgba(30, 41, 59, 0.8);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 0.85rem 1.1rem;
            color: var(--text-main);
            font-family: 'Inter', sans-serif;
            font-size: 0.95rem;
            resize: none;
            height: 60px;
            outline: none;
            transition: all 0.2s ease;
        }

        textarea:focus {
            border-color: var(--accent-a);
            box-shadow: 0 0 0 3px var(--accent-a-glow);
        }

        .btn-send {
            background: linear-gradient(135deg, var(--accent-a), var(--accent-b));
            color: #fff;
            border: none;
            padding: 0 1.8rem;
            height: 60px;
            border-radius: 12px;
            font-weight: 700;
            font-size: 0.95rem;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            box-shadow: 0 4px 15px var(--accent-a-glow);
        }

        .btn-send:hover {
            transform: translateY(-2px);
            box-shadow: 0 6px 20px var(--accent-a-glow);
        }

        .btn-send:disabled {
            opacity: 0.5;
            cursor: not-allowed;
            transform: none;
        }

        .spinner {
            width: 16px;
            height: 16px;
            border: 2px solid rgba(255,255,255,0.3);
            border-top-color: #fff;
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
            display: inline-block;
        }

        @keyframes spin { to { transform: rotate(360deg); } }

        /* LLM Request Inspector Modal */
        .modal-overlay {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(15, 23, 42, 0.88);
            backdrop-filter: blur(12px);
            z-index: 1000;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 1.5rem;
        }

        .modal-card {
            background: #1e293b;
            border: 1px solid var(--border-card);
            border-radius: 16px;
            max-width: 850px;
            width: 100%;
            max-height: 85vh;
            display: flex;
            flex-direction: column;
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
        }

        .modal-header {
            padding: 1.25rem 1.5rem;
            border-bottom: 1px solid var(--border-card);
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .modal-title {
            font-family: 'Outfit', sans-serif;
            font-size: 1.15rem;
            font-weight: 700;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }

        .modal-close {
            background: none;
            border: none;
            color: var(--text-muted);
            font-size: 1.4rem;
            cursor: pointer;
            padding: 0.2rem;
        }

        .modal-subtitle {
            padding: 0.75rem 1.5rem;
            font-size: 0.82rem;
            color: var(--text-muted);
            background: rgba(15, 23, 42, 0.4);
            border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .modal-body {
            padding: 1.5rem;
            overflow-y: auto;
            display: flex;
            flex-direction: column;
            gap: 1rem;
            flex: 1;
        }

        .message-item {
            border-radius: 10px;
            padding: 0.9rem 1.1rem;
            border-left: 4px solid #fff;
            font-size: 0.9rem;
            line-height: 1.5;
        }

        .message-role-tag {
            font-family: 'Outfit', sans-serif;
            font-weight: 700;
            font-size: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-bottom: 0.4rem;
            display: flex;
            justify-content: space-between;
        }

        .msg-role-system {
            background: rgba(99, 102, 241, 0.12);
            border-left-color: #818cf8;
        }
        .msg-role-system .message-role-tag { color: #818cf8; }

        .msg-role-history-user {
            background: rgba(56, 189, 248, 0.12);
            border-left-color: #38bdf8;
        }
        .msg-role-history-user .message-role-tag { color: #38bdf8; }

        .msg-role-history-assistant {
            background: rgba(52, 211, 153, 0.12);
            border-left-color: #34d399;
        }
        .msg-role-history-assistant .message-role-tag { color: #34d399; }

        .msg-role-current-user {
            background: rgba(244, 63, 94, 0.12);
            border-left-color: #f43f5e;
        }
        .msg-role-current-user .message-role-tag { color: #f43f5e; }

        .modal-footer {
            padding: 1rem 1.5rem;
            border-top: 1px solid var(--border-card);
            display: flex;
            justify-content: space-between;
            gap: 1rem;
        }

        .btn-copy {
            background: rgba(99, 102, 241, 0.2);
            color: #a5b4fc;
            border: 1px solid rgba(99, 102, 241, 0.4);
            padding: 0.55rem 1.1rem;
            border-radius: 8px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
        }
        .btn-copy:hover {
            background: rgba(99, 102, 241, 0.35);
            border-color: #6366f1;
        }

        .btn-close-modal {
            background: rgba(255, 255, 255, 0.08);
            color: var(--text-main);
            border: 1px solid var(--border-card);
            padding: 0.55rem 1.1rem;
            border-radius: 8px;
            font-weight: 600;
            cursor: pointer;
        }
    </style>
</head>
<body>

    <header>
        <div class="brand">
            <span class="brand-icon">⚡</span>
            <span class="brand-title">LLM Parallel Exploration Studio</span>
        </div>
        <div class="header-actions">
            <button class="btn-header" onclick="toggleControlsBar()">
                <span id="toggle-models-icon">▲</span>
                <span>Models & System</span>
            </button>
            <button class="btn-header btn-reset" onclick="resetConversation()">
                🔄 Reset Context
            </button>
        </div>
    </header>

    <!-- Collapsible Controls Bar -->
    <div class="controls-bar" id="controls-bar">
        <div class="controls-summary">
            <span id="summary-models-text">Model A: openrouter/free | Model B: google/gemma-4-31b-it:free</span>
            <span style="cursor:pointer; text-decoration:underline;" onclick="toggleControlsBar()">Edit Settings</span>
        </div>

        <div class="model-grid">
            <!-- Model A Selector -->
            <div class="model-selector-card model-a">
                <div class="selector-header">
                    <span class="selector-label">Model A</span>
                </div>
                <select id="select-model-a" onchange="onModelSelectChange('a')"></select>
                <input type="text" id="custom-model-a" placeholder="Enter custom model string..." style="display:none; margin-top:0.4rem;" />
                <div class="model-description" id="desc-model-a">Loading models from config...</div>
                <span class="custom-model-toggle" onclick="toggleCustomModel('a')">Toggle Custom Model Input</span>
            </div>

            <!-- Model B Selector -->
            <div class="model-selector-card model-b">
                <div class="selector-header">
                    <span class="selector-label">Model B</span>
                </div>
                <select id="select-model-b" onchange="onModelSelectChange('b')"></select>
                <input type="text" id="custom-model-b" placeholder="Enter custom model string..." style="display:none; margin-top:0.4rem;" />
                <div class="model-description" id="desc-model-b">Loading models from config...</div>
                <span class="custom-model-toggle" onclick="toggleCustomModel('b')">Toggle Custom Model Input</span>
            </div>
        </div>
    </div>

    <main>
        <!-- System Prompt Collapsible -->
        <div class="system-prompt-card" id="system-prompt-card">
            <div class="system-prompt-header" onclick="toggleSystemPromptCard()">
                <span>SYSTEM INSTRUCTIONS</span>
                <span id="system-prompt-icon" style="font-size:0.75rem; color:#64748b;">▲ Collapse</span>
            </div>
            <div class="system-prompt-body">
                <input type="text" id="system-prompt" placeholder="Optional system prompt (e.g. 'You are a concise security research analyst...')" />
            </div>
        </div>

        <!-- Conversation Turns Container -->
        <div class="turns-container" id="turns-container">
            <!-- Dynamic Turns Will Render Here -->
        </div>
    </main>

    <!-- Bottom Fixed Input Bar -->
    <div class="input-bar-container">
        <div class="input-bar">
            <textarea id="user-prompt" placeholder="Type your prompt here... (Press Ctrl+Enter to send)" onkeydown="if(event.ctrlKey && event.keyCode === 13) sendPrompt();"></textarea>
            <button class="btn-send" id="btn-send" onclick="sendPrompt()">
                <span>Send to Both LLMs</span>
                <span id="btn-spinner" class="spinner" style="display:none;"></span>
            </button>
        </div>
    </div>

    <!-- LLM Request Inspector Modal -->
    <div class="modal-overlay" id="inspector-modal" style="display:none;" onclick="if(event.target === this) closeInspectorModal()">
        <div class="modal-card">
            <div class="modal-header">
                <div class="modal-title">
                    <span>🔍 Input Prompt &amp; Context Inspector</span>
                    <span id="modal-model-badge" class="pill"></span>
                </div>
                <button class="modal-close" onclick="closeInspectorModal()">✕</button>
            </div>
            <div class="modal-subtitle">
                Exact payload sent to the LLM endpoint for this turn. Messages are color-coded by role and conversation turn.
            </div>
            <div class="modal-body" id="modal-messages-container">
                <!-- Messages rendered dynamically -->
            </div>
            <div class="modal-footer">
                <button class="btn-copy" id="btn-copy-json" onclick="copyInspectorJson()">
                    <span>📋 Copy Raw JSON Payload</span>
                </button>
                <button class="btn-close-modal" onclick="closeInspectorModal()">Close</button>
            </div>
        </div>
    </div>

    <script>
        let availableModels = [];
        let history = []; // Shared conversation history: [{user: "...", assistant: "..."}]
        let turnResults = {}; // Map of turnId -> { model_a, model_b, prompt, messages }
        let currentTurnIndex = 0;
        let autoSelectedSide = 'a'; // Auto-select preferred branch for subsequent rounds
        let isCustomA = false;
        let isCustomB = false;
        let currentInspectorJson = "";

        async function init() {
            try {
                const res = await fetch('/api/models');
                if (res.ok) {
                    const data = await res.json();
                    if (data.models && data.models.length > 0) {
                        availableModels = data.models;
                        populateModelSelects();
                    }
                }
            } catch(e) {
                console.log("Using default model dropdowns");
            }
        }

        function populateModelSelects() {
            ['a', 'b'].forEach(side => {
                const sel = document.getElementById(`select-model-${side}`);
                sel.innerHTML = '';
                availableModels.forEach(m => {
                    const opt = document.createElement('option');
                    opt.value = m.name;
                    opt.textContent = m.display_name;
                    sel.appendChild(opt);
                });
            });

            const selA = document.getElementById('select-model-a');
            const selB = document.getElementById('select-model-b');

            const idxA = availableModels.findIndex(m => m.name === 'openrouter/free');
            if (idxA !== -1) selA.selectedIndex = idxA;

            const idxB = availableModels.findIndex(m => m.name === 'google/gemma-4-26b-a4b-it:free');
            if (idxB !== -1) selB.selectedIndex = idxB;
            else if (availableModels.length > 1) selB.selectedIndex = 1;

            onModelSelectChange('a');
            onModelSelectChange('b');
        }

        function onModelSelectChange(side) {
            const val = document.getElementById(`select-model-${side}`).value;
            const modelObj = availableModels.find(m => m.name === val);
            const descEl = document.getElementById(`desc-model-${side}`);
            if (modelObj && modelObj.description) {
                descEl.textContent = modelObj.description;
            } else {
                descEl.textContent = val;
            }
            updateSummaryText();
        }

        function updateSummaryText() {
            const mA = getSelectedModel('a');
            const mB = getSelectedModel('b');
            document.getElementById('summary-models-text').textContent = `Model A: ${mA} | Model B: ${mB}`;
        }

        function toggleControlsBar() {
            const bar = document.getElementById('controls-bar');
            const icon = document.getElementById('toggle-models-icon');
            bar.classList.toggle('collapsed');
            icon.textContent = bar.classList.contains('collapsed') ? '▼' : '▲';
        }

        function toggleSystemPromptCard() {
            const card = document.getElementById('system-prompt-card');
            const icon = document.getElementById('system-prompt-icon');
            card.classList.toggle('collapsed');
            icon.textContent = card.classList.contains('collapsed') ? '▼ Expand' : '▲ Collapse';
        }

        function toggleCustomModel(side) {
            const isCustom = side === 'a' ? !isCustomA : !isCustomB;
            if (side === 'a') isCustomA = isCustom; else isCustomB = isCustom;

            const selectEl = document.getElementById(`select-model-${side}`);
            const inputEl = document.getElementById(`custom-model-${side}`);
            if (isCustom) {
                selectEl.style.display = 'none';
                inputEl.style.display = 'block';
            } else {
                selectEl.style.display = 'block';
                inputEl.style.display = 'none';
            }
            updateSummaryText();
        }

        function getSelectedModel(side) {
            const isCustom = side === 'a' ? isCustomA : isCustomB;
            if (isCustom) {
                return document.getElementById(`custom-model-${side}`).value.trim() || "default";
            }
            return document.getElementById(`select-model-${side}`).value;
        }

        function setModelSelectsEnabled(enabled) {
            document.getElementById('select-model-a').disabled = !enabled;
            document.getElementById('select-model-b').disabled = !enabled;
            document.getElementById('custom-model-a').disabled = !enabled;
            document.getElementById('custom-model-b').disabled = !enabled;
            
            const toggles = document.querySelectorAll('.custom-model-toggle');
            toggles.forEach(el => {
                el.style.pointerEvents = enabled ? 'auto' : 'none';
                el.style.opacity = enabled ? '1' : '0.4';
            });
        }

        function resetConversation() {
            history = [];
            turnResults = {};
            currentTurnIndex = 0;
            autoSelectedSide = 'a';
            setModelSelectsEnabled(true);
            document.getElementById('turns-container').innerHTML = '';
            document.getElementById('user-prompt').value = '';
            document.getElementById('controls-bar').classList.remove('collapsed');
            document.getElementById('system-prompt-card').classList.remove('collapsed');
        }

        function buildTurnMessagesPayload(turnId, currentPromptText) {
            const systemPrompt = document.getElementById('system-prompt').value.trim();
            const messages = [];
            if (systemPrompt) {
                messages.push({ role: 'system', content: systemPrompt });
            }
            for (let i = 0; i < turnId; i++) {
                if (history[i]) {
                    messages.push({ role: 'user', content: history[i].user });
                    messages.push({ role: 'assistant', content: history[i].assistant });
                }
            }
            messages.push({ role: 'user', content: currentPromptText });
            return messages;
        }

        async function sendPrompt() {
            const promptInput = document.getElementById('user-prompt');
            const promptText = promptInput.value.trim();
            if (!promptText) return;

            const modelA = getSelectedModel('a');
            const modelB = getSelectedModel('b');
            const systemPrompt = document.getElementById('system-prompt').value.trim();

            // Auto collapse top bars once user sends prompt
            document.getElementById('controls-bar').classList.add('collapsed');
            if (systemPrompt) {
                document.getElementById('system-prompt-card').classList.add('collapsed');
            }

            const sendBtn = document.getElementById('btn-send');
            const spinner = document.getElementById('btn-spinner');
            sendBtn.disabled = true;
            spinner.style.display = 'inline-block';

            // Create Turn Block UI placeholder
            const turnId = currentTurnIndex;
            const turnBlock = document.createElement('div');
            turnBlock.className = 'turn-block';
            turnBlock.id = `turn-${turnId}`;
            turnBlock.innerHTML = `
                <div class="turn-prompt-badge">
                    <span>Turn #${turnId + 1}:</span>
                    <span style="color:#e2e8f0; font-weight:400;">${escapeHtml(promptText)}</span>
                </div>
                <div class="dual-grid">
                    <div class="response-card card-a" id="card-a-${turnId}">
                        <div class="card-header">
                            <span class="model-badge">Model A: ${escapeHtml(modelA)}</span>
                        </div>
                        <div class="response-body"><span class="spinner"></span> Generating response...</div>
                    </div>
                    <div class="response-card card-b" id="card-b-${turnId}">
                        <div class="card-header">
                            <span class="model-badge">Model B: ${escapeHtml(modelB)}</span>
                        </div>
                        <div class="response-body"><span class="spinner"></span> Generating response...</div>
                    </div>
                </div>
            `;
            document.getElementById('turns-container').appendChild(turnBlock);
            turnBlock.scrollIntoView({ behavior: 'smooth' });

            const payloadMessages = buildTurnMessagesPayload(turnId, promptText);

            try {
                const response = await fetch('/api/compare', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        model_a: modelA,
                        model_b: modelB,
                        history: history,
                        prompt: promptText,
                        system_prompt: systemPrompt
                    })
                });

                const data = await response.json();
                renderTurnResults(turnId, data, promptText, payloadMessages);
            } catch (err) {
                renderTurnError(turnId, err.message);
            } finally {
                sendBtn.disabled = false;
                spinner.style.display = 'none';
                promptInput.value = '';
                currentTurnIndex++;
                setTimeout(() => {
                    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
                    promptInput.focus();
                }, 150);
            }
        }

        function renderTurnResults(turnId, data, promptText, payloadMessages) {
            const resA = data.model_a || {};
            const resB = data.model_b || {};

            turnResults[turnId] = {
                model_a: resA,
                model_b: resB,
                prompt: promptText,
                messages: payloadMessages || buildTurnMessagesPayload(turnId, promptText)
            };

            renderCard('a', turnId, resA);
            renderCard('b', turnId, resB);

            // Lock model selectors after at least one successful generation
            if (resA.status !== 'error' || resB.status !== 'error') {
                setModelSelectsEnabled(false);
            }

            // Auto-select preferred branch if it succeeded; if it failed, pick the other non-errored branch
            let sideToSelect = autoSelectedSide;
            if (sideToSelect === 'a' && resA.status === 'error' && resB.status !== 'error') {
                sideToSelect = 'b';
            } else if (sideToSelect === 'b' && resB.status === 'error' && resA.status !== 'error') {
                sideToSelect = 'a';
            }

            if ((sideToSelect === 'a' && resA.status !== 'error') || (sideToSelect === 'b' && resB.status !== 'error')) {
                selectContext(turnId, sideToSelect);
            }
        }

        function renderCard(side, turnId, res) {
            const cardEl = document.getElementById(`card-${side}-${turnId}`);
            const tokens = res.tokens || { prompt: 0, completion: 0, total: 0 };
            const latency = res.latency_ms || 0;
            const isError = res.status === 'error';

            if (isError) {
                cardEl.classList.add('card-errored');
            } else {
                cardEl.classList.remove('card-errored');
            }

            const modelTitle = res.routed_model && res.routed_model !== res.model_name ? 
                `${res.model_name} → ${res.routed_model}` : (res.model_name || '');

            cardEl.innerHTML = `
                <div class="card-header">
                    <span class="model-badge">Model ${side.toUpperCase()}: ${escapeHtml(modelTitle)}</span>
                    ${res.provider && res.provider !== 'N/A' ? `<span class="pill">${escapeHtml(res.provider)}</span>` : ''}
                </div>
                <div class="response-body">${escapeHtml(res.content || '')}</div>
                <div class="meta-pills">
                    <span class="pill latency" title="Execution Latency: Time taken in milliseconds for response generation">⚡ ${latency} ms</span>
                    <span class="pill tokens pill-clickable" title="Click to inspect raw color-coded LLM request payload &amp; context history" onclick="openInspectorModal(${turnId}, '${side}')">📥 ${tokens.prompt} in 🔍</span>
                    <span class="pill tokens" title="Output Tokens: Number of response completion tokens generated by the LLM">📤 ${tokens.completion} out</span>
                    <span class="pill tokens" title="Total Tokens: Combined total of input and output tokens consumed in this turn">📊 ${tokens.total} total</span>
                </div>
                <div class="card-actions">
                    ${isError ? `
                        <button class="context-select-btn" disabled>
                            <span>❌ Errored Branch (Selection Disabled)</span>
                        </button>
                        <button class="btn-retry-model" onclick="retrySingleModel(${turnId}, '${side}')">
                            <span>🔄 Retry Model ${side.toUpperCase()} Only</span>
                        </button>
                    ` : `
                        <button class="context-select-btn" id="btn-ctx-${side}-${turnId}" onclick="selectContext(${turnId}, '${side}')">
                            <span>Use Model ${side.toUpperCase()}'s Answer for Next Turn Context</span>
                        </button>
                    `}
                </div>
            `;
        }

        function selectContext(turnId, chosenSide) {
            const turnData = turnResults[turnId];
            if (!turnData) return;

            const chosenRes = chosenSide === 'a' ? turnData.model_a : turnData.model_b;
            if (chosenRes.status === 'error') return; // Do not allow selection of failed branch

            // Update auto-selected preferred side for subsequent rounds
            autoSelectedSide = chosenSide;

            // Highlight selected card
            document.getElementById(`card-a-${turnId}`).classList.remove('selected-context');
            document.getElementById(`card-b-${turnId}`).classList.remove('selected-context');
            document.getElementById(`card-${chosenSide}-${turnId}`).classList.add('selected-context');

            // Update button labels
            const btnA = document.getElementById(`btn-ctx-a-${turnId}`);
            const btnB = document.getElementById(`btn-ctx-b-${turnId}`);
            if (btnA && !btnA.disabled) btnA.innerHTML = '<span>Use Model A Context</span>';
            if (btnB && !btnB.disabled) btnB.innerHTML = '<span>Use Model B Context</span>';

            const activeBtn = document.getElementById(`btn-ctx-${chosenSide}-${turnId}`);
            if (activeBtn) {
                activeBtn.innerHTML = `<span>✓ Selected Model ${chosenSide.toUpperCase()} Context for Next Turn</span>`;
            }

            // Update shared conversation history
            history[turnId] = {
                user: turnData.prompt,
                assistant: chosenRes.content || ''
            };
        }

        async function retrySingleModel(turnId, side) {
            const turnData = turnResults[turnId];
            if (!turnData) return;

            const modelName = side === 'a' ? getSelectedModel('a') : getSelectedModel('b');
            const messages = buildTurnMessagesPayload(turnId, turnData.prompt);

            const cardEl = document.getElementById(`card-${side}-${turnId}`);
            cardEl.innerHTML = `
                <div class="card-header"><span class="model-badge">Model ${side.toUpperCase()}: ${escapeHtml(modelName)}</span></div>
                <div class="response-body"><span class="spinner"></span> Retrying ${escapeHtml(modelName)}...</div>
            `;

            try {
                const res = await fetch('/api/compare_single', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        model_name: modelName,
                        messages: messages
                    })
                });

                const resData = await res.json();
                if (side === 'a') turnData.model_a = resData; else turnData.model_b = resData;
                turnData.messages = messages;
                renderCard(side, turnId, resData);

                if (resData.status !== 'error') {
                    selectContext(turnId, side);
                }
            } catch (err) {
                renderCard(side, turnId, { status: 'error', content: err.message, model_name: modelName });
            }
        }

        function openInspectorModal(turnId, side) {
            const turnData = turnResults[turnId];
            if (!turnData) return;

            const res = side === 'a' ? turnData.model_a : turnData.model_b;
            const messages = turnData.messages || buildTurnMessagesPayload(turnId, turnData.prompt);

            currentInspectorJson = JSON.stringify(messages, null, 2);

            const badgeEl = document.getElementById('modal-model-badge');
            badgeEl.textContent = `Model ${side.toUpperCase()}: ${res.model_name || ''}`;

            const container = document.getElementById('modal-messages-container');
            container.innerHTML = '';

            const hasSystem = messages.length > 0 && messages[0].role === 'system';

            messages.forEach((msg, idx) => {
                const item = document.createElement('div');
                item.className = 'message-item';

                let roleLabel = '';
                if (msg.role === 'system') {
                    item.classList.add('msg-role-system');
                    roleLabel = '⚙ SYSTEM INSTRUCTION';
                } else {
                    const offsetIdx = hasSystem ? idx - 1 : idx;
                    const turnNum = Math.floor(offsetIdx / 2) + 1;

                    if (idx === messages.length - 1 && msg.role === 'user') {
                        item.classList.add('msg-role-current-user');
                        roleLabel = `🚀 CURRENT USER PROMPT (TURN #${turnNum})`;
                    } else if (msg.role === 'user') {
                        item.classList.add('msg-role-history-user');
                        roleLabel = `👤 USER PROMPT (PAST TURN #${turnNum})`;
                    } else if (msg.role === 'assistant') {
                        item.classList.add('msg-role-history-assistant');
                        roleLabel = `🤖 ASSISTANT RESPONSE (PAST TURN #${turnNum})`;
                    }
                }

                item.innerHTML = `
                    <div class="message-role-tag">
                        <span>${roleLabel}</span>
                        <span>${msg.content ? msg.content.length : 0} chars</span>
                    </div>
                    <div style="white-space: pre-wrap; font-family: monospace; font-size: 0.88rem; color: #f1f5f9;">${escapeHtml(msg.content || '')}</div>
                `;
                container.appendChild(item);
            });

            document.getElementById('inspector-modal').style.display = 'flex';
        }

        function closeInspectorModal() {
            document.getElementById('inspector-modal').style.display = 'none';
        }

        function copyInspectorJson() {
            if (!navigator.clipboard) return;
            navigator.clipboard.writeText(currentInspectorJson).then(() => {
                const btn = document.getElementById('btn-copy-json');
                btn.textContent = '✓ Copied Raw JSON Payload!';
                setTimeout(() => { btn.textContent = '📋 Copy Raw JSON Payload'; }, 2000);
            });
        }

        function escapeHtml(str) {
            if (!str) return '';
            return String(str)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#039;');
        }

        window.onload = init;
    </script>
</body>
</html>
"""


@app.route('/')
def index():
    return render_template_string(HTML_TEMPLATE)


@app.route('/api/models', methods=['GET'])
def get_models():
    try:
        resp = requests.get(f"{SERVICE_URL}/models", timeout=5)
        return jsonify(resp.json())
    except Exception as e:
        print(f"[!] Error fetching models from service: {e}")
        models_file = Path(__file__).parent / "service" / "models.json"
        if models_file.exists():
            with open(models_file, "r", encoding="utf-8") as f:
                return jsonify({"models": json.load(f)})
        return jsonify({"models": []})


@app.route('/api/compare', methods=['POST'])
def compare():
    data = request.json or {}
    try:
        resp = requests.post(
            f"{SERVICE_URL}/compare",
            json=data,
            timeout=70
        )
        return jsonify(resp.json())
    except Exception as e:
        print(f"[!] Service connection error: {e}")
        return jsonify({"error": "Service error: Unable to connect to challenge service."}), 500


@app.route('/api/compare_single', methods=['POST'])
def compare_single():
    data = request.json or {}
    try:
        resp = requests.post(
            f"{SERVICE_URL}/compare_single",
            json=data,
            timeout=70
        )
        return jsonify(resp.json())
    except Exception as e:
        print(f"[!] Service connection error: {e}")
        return jsonify({"error": "Service error: Unable to connect to challenge service."}), 500


if __name__ == '__main__':
    port = int(os.environ.get("PORT", 5000))
    print(f"[*] Parallel LLM Exploration UI running on port {port}")
    app.run(host='0.0.0.0', port=port)
