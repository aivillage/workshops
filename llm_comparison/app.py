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
            background: rgba(15, 23, 42, 0.9);
            backdrop-filter: blur(12px);
            border-bottom: 1px solid var(--border-card);
            padding: 1rem 2rem;
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
            font-size: 1.8rem;
            background: linear-gradient(135deg, var(--accent-a), var(--accent-b));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .brand-title {
            font-family: 'Outfit', sans-serif;
            font-size: 1.4rem;
            font-weight: 700;
            letter-spacing: -0.02em;
        }

        .header-actions {
            display: flex;
            gap: 1rem;
            align-items: center;
        }

        .btn-reset {
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
            border: 1px solid rgba(239, 68, 68, 0.3);
            padding: 0.5rem 1.2rem;
            border-radius: 8px;
            font-weight: 600;
            font-size: 0.88rem;
            cursor: pointer;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .btn-reset:hover {
            background: rgba(239, 68, 68, 0.3);
            border-color: #ef4444;
            transform: translateY(-1px);
        }

        /* Controls Section */
        .controls-bar {
            background: rgba(30, 41, 59, 0.4);
            border-bottom: 1px solid var(--border-card);
            padding: 1.25rem 2rem;
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 2rem;
        }

        .model-selector-card {
            background: var(--bg-card);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 1rem 1.25rem;
            transition: border-color 0.3s ease;
        }

        .model-selector-card.model-a { border-left: 4px solid var(--accent-a); }
        .model-selector-card.model-b { border-left: 4px solid var(--accent-b); }

        .selector-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.6rem;
        }

        .selector-label {
            font-family: 'Outfit', sans-serif;
            font-weight: 700;
            font-size: 0.95rem;
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
            padding: 0.65rem 0.85rem;
            font-family: 'Inter', sans-serif;
            font-size: 0.9rem;
            outline: none;
            transition: all 0.2s ease;
        }

        select:focus, input[type="text"]:focus {
            border-color: var(--accent-a);
            box-shadow: 0 0 0 3px var(--accent-a-glow);
        }

        .model-description {
            font-size: 0.8rem;
            color: var(--text-muted);
            margin-top: 0.5rem;
            line-height: 1.4;
        }

        .custom-model-toggle {
            font-size: 0.78rem;
            color: var(--text-muted);
            cursor: pointer;
            text-decoration: underline;
            margin-top: 0.4rem;
            display: inline-block;
        }

        /* Main Dual Stream Layout */
        main {
            flex: 1;
            padding: 1.5rem 2rem 140px;
            overflow-y: auto;
            max-width: 1600px;
            width: 100%;
            margin: 0 auto;
        }

        .system-prompt-card {
            background: var(--bg-card);
            border: 1px solid var(--border-card);
            border-radius: 12px;
            padding: 1rem 1.25rem;
            margin-bottom: 1.5rem;
        }

        .system-prompt-header {
            font-size: 0.85rem;
            font-weight: 600;
            color: var(--text-muted);
            margin-bottom: 0.5rem;
            display: flex;
            justify-content: space-between;
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

        /* Context Selection Control */
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

        .context-select-btn:hover {
            background: rgba(255, 255, 255, 0.12);
            color: var(--text-main);
        }

        .selected-context .context-select-btn {
            background: var(--accent-emerald);
            color: #000;
            border-color: var(--accent-emerald);
            font-weight: 700;
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

        /* Loading Spinner */
        .spinner {
            width: 18px;
            height: 18px;
            border: 2px solid rgba(255,255,255,0.3);
            border-top-color: #fff;
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
            display: inline-block;
        }

        @keyframes spin { to { transform: rotate(360deg); } }
    </style>
</head>
<body>

    <header>
        <div class="brand">
            <span class="brand-icon">⚡</span>
            <span class="brand-title">LLM Parallel Exploration Studio</span>
        </div>
        <div class="header-actions">
            <button class="btn-reset" onclick="resetConversation()">
                🔄 Reset Context
            </button>
        </div>
    </header>

    <div class="controls-bar">
        <!-- Model A Selector -->
        <div class="model-selector-card model-a">
            <div class="selector-header">
                <span class="selector-label">Model A</span>
            </div>
            <select id="select-model-a" onchange="onModelSelectChange('a')">
                <option value="google/gemma-2-9b-it:free">Gemma 2 9B (Google)</option>
                <option value="meta-llama/llama-3.3-70b-instruct:free">Llama 3.3 70B (Meta)</option>
                <option value="deepseek/deepseek-r1:free">DeepSeek R1 (Reasoning)</option>
                <option value="qwen/qwen-2.5-coder-32b-instruct:free">Qwen 2.5 Coder 32B (Alibaba)</option>
                <option value="mistralai/mistral-7b-instruct:free">Mistral 7B (Mistral AI)</option>
                <option value="nvidia/nemotron-nano-9b-v2:free">Nemotron Nano 9B (Nvidia)</option>
                <option value="inclusionai/ling-3.0-tiny:free">Ling 3.0 Tiny (Ant Group)</option>
            </select>
            <input type="text" id="custom-model-a" placeholder="Enter custom model string..." style="display:none; margin-top:0.5rem;" />
            <div class="model-description" id="desc-model-a">High-efficiency instruction-tuned model developed by Google DeepMind.</div>
            <span class="custom-model-toggle" onclick="toggleCustomModel('a')">Toggle Custom Model Input</span>
        </div>

        <!-- Model B Selector -->
        <div class="model-selector-card model-b">
            <div class="selector-header">
                <span class="selector-label">Model B</span>
            </div>
            <select id="select-model-b" onchange="onModelSelectChange('b')">
                <option value="meta-llama/llama-3.3-70b-instruct:free">Llama 3.3 70B (Meta)</option>
                <option value="google/gemma-2-9b-it:free">Gemma 2 9B (Google)</option>
                <option value="deepseek/deepseek-r1:free">DeepSeek R1 (Reasoning)</option>
                <option value="qwen/qwen-2.5-coder-32b-instruct:free">Qwen 2.5 Coder 32B (Alibaba)</option>
                <option value="mistralai/mistral-7b-instruct:free">Mistral 7B (Mistral AI)</option>
                <option value="nvidia/nemotron-nano-9b-v2:free">Nemotron Nano 9B (Nvidia)</option>
                <option value="inclusionai/ling-3.0-tiny:free">Ling 3.0 Tiny (Ant Group)</option>
            </select>
            <input type="text" id="custom-model-b" placeholder="Enter custom model string..." style="display:none; margin-top:0.5rem;" />
            <div class="model-description" id="desc-model-b">State-of-the-art open weights 70B parameter model by Meta for complex reasoning.</div>
            <span class="custom-model-toggle" onclick="toggleCustomModel('b')">Toggle Custom Model Input</span>
        </div>
    </div>

    <main>
        <!-- System Prompt Collapsible -->
        <div class="system-prompt-card">
            <div class="system-prompt-header">
                <span>SYSTEM INSTRUCTIONS</span>
                <span style="font-size:0.75rem; color:#64748b;">Applied to both models</span>
            </div>
            <input type="text" id="system-prompt" placeholder="Optional system prompt (e.g. 'You are a concise security research analyst...')" />
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

    <script>
        let availableModels = [];
        let history = []; // List of turn objects: [{user: "...", assistant: "..."}]
        let currentTurnIndex = 0;
        let isCustomA = false;
        let isCustomB = false;

        // Fetch models catalog on load
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
            if (availableModels.length > 1) {
                document.getElementById('select-model-b').selectedIndex = 1;
            }
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
        }

        function getSelectedModel(side) {
            const isCustom = side === 'a' ? isCustomA : isCustomB;
            if (isCustom) {
                return document.getElementById(`custom-model-${side}`).value.trim() || "default";
            }
            return document.getElementById(`select-model-${side}`).value;
        }

        function resetConversation() {
            history = [];
            currentTurnIndex = 0;
            document.getElementById('turns-container').innerHTML = '';
            document.getElementById('user-prompt').value = '';
        }

        async function sendPrompt() {
            const promptInput = document.getElementById('user-prompt');
            const promptText = promptInput.value.trim();
            if (!promptText) return;

            const modelA = getSelectedModel('a');
            const modelB = getSelectedModel('b');
            const systemPrompt = document.getElementById('system-prompt').value.trim();

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
                renderTurnResults(turnId, data, promptText);
            } catch (err) {
                renderTurnError(turnId, err.message);
            } finally {
                sendBtn.disabled = false;
                spinner.style.display = 'none';
                promptInput.value = '';
                currentTurnIndex++;
            }
        }

        function renderTurnResults(turnId, data, promptText) {
            const resA = data.model_a || {};
            const resB = data.model_b || {};

            renderCard('a', turnId, resA, promptText);
            renderCard('b', turnId, resB, promptText);

            // Default select Model A response as history context if first turn
            selectContext(turnId, 'a', promptText, resA.content);
        }

        function renderCard(side, turnId, res, promptText) {
            const cardEl = document.getElementById(`card-${side}-${turnId}`);
            const tokens = res.tokens || { prompt: 0, completion: 0, total: 0 };
            const latency = res.latency_ms || 0;

            cardEl.innerHTML = `
                <div class="card-header">
                    <span class="model-badge">Model ${side.toUpperCase()}: ${escapeHtml(res.model_name || '')}</span>
                </div>
                <div class="response-body">${escapeHtml(res.content || '')}</div>
                <div class="meta-pills">
                    <span class="pill latency">⚡ ${latency} ms</span>
                    <span class="pill tokens">📥 ${tokens.prompt} in</span>
                    <span class="pill tokens">📤 ${tokens.completion} out</span>
                    <span class="pill tokens">📊 ${tokens.total} total</span>
                </div>
                <button class="context-select-btn" id="btn-ctx-${side}-${turnId}" onclick="selectContext(${turnId}, '${side}', '${escapeJs(promptText)}', '${escapeJs(res.content || '')}')">
                    <span>Use Model ${side.toUpperCase()}'s Answer for Next Turn Context</span>
                </button>
            `;
        }

        function selectContext(turnId, chosenSide, userPrompt, assistantReply) {
            // Highlight selected card
            document.getElementById(`card-a-${turnId}`).classList.remove('selected-context');
            document.getElementById(`card-b-${turnId}`).classList.remove('selected-context');
            document.getElementById(`card-${chosenSide}-${turnId}`).classList.add('selected-context');

            // Update button labels
            document.getElementById(`btn-ctx-a-${turnId}`).innerHTML = '<span>Use Model A Context</span>';
            document.getElementById(`btn-ctx-b-${turnId}`).innerHTML = '<span>Use Model B Context</span>';
            document.getElementById(`btn-ctx-${chosenSide}-${turnId}`).innerHTML = '<span>✓ Selected Context for Next Turn</span>';

            // Store in history array
            history[turnId] = {
                user: userPrompt,
                assistant: assistantReply
            };
        }

        function renderTurnError(turnId, errMsg) {
            ['a', 'b'].forEach(side => {
                const cardEl = document.getElementById(`card-${side}-${turnId}`);
                cardEl.innerHTML = `<div class="response-body" style="color:#f87171;">Error: ${escapeHtml(errMsg)}</div>`;
            });
        }

        function escapeHtml(str) {
            return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
        }

        function escapeJs(str) {
            return String(str).replace(/\\\\/g, '\\\\\\\\').replace(/'/g, "\\\\'");
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


if __name__ == '__main__':
    port = int(os.environ.get("PORT", 5000))
    print(f"[*] Parallel LLM Exploration UI running on port {port}")
    app.run(host='0.0.0.0', port=port)
