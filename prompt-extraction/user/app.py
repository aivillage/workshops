import os
import requests
from flask import Flask, render_template_string, request, jsonify

app = Flask(__name__)

SERVICE_URL = os.environ.get("SERVICE_URL", "http://prompt-extraction-service:5000")

HTML_TEMPLATE = """
<!DOCTYPE html>
<html>
<head>
    <title>OmniCorp AI Assistant</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: monospace;
            background: #0a0a0a;
            color: #00ff41;
            height: 100vh;
            display: flex;
            flex-direction: column;
        }
        .header {
            background: #111;
            border-bottom: 1px solid #333;
            padding: 1rem 2rem;
        }
        .header h1 { font-size: 1.4em; }
        .header p { color: #666; font-size: 0.85em; margin-top: 4px; }
        .objective {
            background: #1a0a0a;
            border: 1px solid #ff4444;
            color: #ff6666;
            padding: 0.8rem 2rem;
            font-size: 0.9em;
        }
        .chat-container {
            flex: 1;
            overflow-y: auto;
            padding: 1rem 2rem;
        }
        .message {
            margin-bottom: 1rem;
            padding: 0.8rem 1rem;
            border-radius: 4px;
            max-width: 80%;
            word-wrap: break-word;
            white-space: pre-wrap;
        }
        .message.user {
            background: #1a2a1a;
            border: 1px solid #2a4a2a;
            margin-left: auto;
            text-align: right;
        }
        .message.assistant {
            background: #111;
            border: 1px solid #333;
            color: #ccc;
        }
        .message .role {
            font-size: 0.75em;
            color: #666;
            margin-bottom: 4px;
        }
        .input-area {
            border-top: 1px solid #333;
            padding: 1rem 2rem;
            background: #111;
            display: flex;
            gap: 0.5rem;
        }
        .input-area textarea {
            flex: 1;
            background: #000;
            color: #00ff41;
            border: 1px solid #333;
            padding: 0.6rem;
            font-family: monospace;
            font-size: 1em;
            resize: none;
            outline: none;
            min-height: 2.5em;
        }
        .input-area textarea:focus { border-color: #00ff41; }
        .input-area button {
            background: #00ff41;
            color: #000;
            border: none;
            padding: 0.6rem 1.5rem;
            font-family: monospace;
            font-weight: bold;
            cursor: pointer;
            font-size: 1em;
        }
        .input-area button:hover { background: #00cc33; }
        .input-area button:disabled { background: #333; color: #666; cursor: wait; }
        .typing { color: #666; font-style: italic; padding: 0.5rem 1rem; }
        .message.victory {
            background: #0a2a0a;
            border: 2px solid #00ff41;
            color: #00ff41;
            text-align: center;
            max-width: 100%;
            font-size: 1.1em;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>OmniCorp AI Assistant</h1>
        <p>Internal Employee Help Desk &mdash; Powered by AI</p>
    </div>
    <div class="objective">
        OBJECTIVE: The AI has a secret authorization code hidden in its instructions. Extract it.
    </div>
    <div class="chat-container" id="chat">
        <div class="message assistant">
            <div class="role">OMNICORP-AI</div>
            Welcome to OmniCorp! I'm your AI assistant. How can I help you today?
        </div>
    </div>
    <div class="input-area">
        <textarea id="input" placeholder="Type your message..." rows="1"
            onkeydown="if(event.key==='Enter' && !event.shiftKey){event.preventDefault();sendMessage();}"></textarea>
        <button id="sendBtn" onclick="sendMessage()">SEND</button>
    </div>
    <script>
        const history = [];
        const chat = document.getElementById('chat');
        const input = document.getElementById('input');
        const sendBtn = document.getElementById('sendBtn');

        async function sendMessage() {
            const msg = input.value.trim();
            if (!msg) return;

            input.value = '';
            sendBtn.disabled = true;

            // Add user message
            addMessage('user', msg);
            history.push({role: 'user', content: msg});

            // Show typing indicator
            const typing = document.createElement('div');
            typing.className = 'typing';
            typing.textContent = 'OmniCorp-AI is thinking...';
            chat.appendChild(typing);
            chat.scrollTop = chat.scrollHeight;

            try {
                const resp = await fetch('/chat', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({message: msg, history: history.slice(0, -1)})
                });
                const data = await resp.json();
                typing.remove();

                addMessage('assistant', data.response);
                history.push({role: 'assistant', content: data.response});

                if (data.won) {
                    const victory = document.createElement('div');
                    victory.className = 'message victory';
                    victory.textContent = 'EXTRACTION SUCCESSFUL! Flag: ' + data.flag;
                    chat.appendChild(victory);
                    chat.scrollTop = chat.scrollHeight;
                }
            } catch (e) {
                typing.remove();
                addMessage('assistant', 'Error: Could not reach the AI service.');
            }

            sendBtn.disabled = false;
            input.focus();
        }

        function addMessage(role, content) {
            const div = document.createElement('div');
            div.className = 'message ' + role;
            const label = document.createElement('div');
            label.className = 'role';
            label.textContent = role === 'user' ? 'YOU' : 'OMNICORP-AI';
            div.appendChild(label);
            div.appendChild(document.createTextNode(content));
            chat.appendChild(div);
            chat.scrollTop = chat.scrollHeight;
        }

        input.focus();
    </script>
</body>
</html>
"""

@app.route('/')
def index():
    return render_template_string(HTML_TEMPLATE)

@app.route('/chat', methods=['POST'])
def chat():
    data = request.json
    try:
        resp = requests.post(
            f"{SERVICE_URL}/chat",
            json=data,
            timeout=65
        )
        return jsonify(resp.json())
    except Exception as e:
        return jsonify({"response": f"Service error: {str(e)}"}), 500

if __name__ == '__main__':
    print("[*] Prompt Extraction UI starting")
    app.run(host='0.0.0.0', port=5000)
