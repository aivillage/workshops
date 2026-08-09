# ⚡ Parallel LLM Exploration & Comparison Studio

An interactive multi-turn exploration tool for testing and comparing two Large Language Models (LLMs) side-by-side in real time.

The **User UI** (`app.py`) provides a dual-column stream interface, and the **Service Backend** (`service/service.py`) executes model requests concurrently using a thread pool.

---

## 🚀 Key Features

* **Parallel Model Execution**: Queries both selected models concurrently via `ThreadPoolExecutor` for zero added latency.
* **External Model Catalogue**: Models and descriptions are configured externally in `service/models.json`.
* **Multi-Turn Context Branching**: Select whether Model A's or Model B's response is kept in the conversation history context for subsequent turns.
* **Auto-Branch Continuation**: The chosen response branch automatically carries forward into future rounds.
* **Single-Model Retry**: If a specific model hits a rate limit or timeout, click **"🔄 Retry Model [A/B] Only"** on that card to retry just that single request without re-running the turn.
* **Color-Coded LLM Request Inspector**: Click the **`📥 Tokens In`** pill on any card to open a color-coded inspection modal displaying the exact system instructions, past turn history, and current prompt payload sent to the LLM.
* **Response Metadata Pills**: Displays execution latency (`ms`), input tokens, output tokens, total tokens, routed model slug, and provider info.
* **Model Selection Locking**: Selectors lock after successful generation to prevent mid-thread model changes; click **"🔄 Reset Context"** to start a fresh thread and unlock selectors.

---

## 🛠️ Architecture

```text
llm_comparison/
├── app.py              # Flask Web UI frontend (runs on port 5000 -> host 30080)
├── Dockerfile          # Container build manifest for UI
├── docker-compose.yml  # Compose file mapping environment & host port 30080
└── service/
    ├── service.py      # Flask API backend executing parallel LLM calls
    ├── models.json     # Configuration file for model selection list & descriptions
    └── Dockerfile      # Container build manifest for backend service
```

---

## 🏁 Quickstart

### 1. Launch with Docker Compose

Run the following command from the `llm_comparison/` directory:

```bash
docker-compose --env-file ~/.config/aiv-workshops/secrets.env up --build
```

### 2. Access the Web UI

Open your browser to:
* **Linux / Mac / Windows**: `http://localhost:30080`
* **Chromebook (Crostini)**: `http://localhost:30080` or `http://penguin.linux.test:30080`

### 3. Launching Locally Without Docker (Direct Python)

1. **Install Dependencies**:
   ```bash
   pip install flask requests
   ```

2. **Start Services** (from repository root):
   ```bash
   # Terminal 1: Start Backend Service (Port 5000)
   PYTHONPATH=. python3 llm_comparison/service/service.py

   # Terminal 2: Start Web UI Frontend (Port 5001)
   SERVICE_URL=http://localhost:5000 PORT=5001 PYTHONPATH=. python3 llm_comparison/app.py
   ```

3. Open browser to **`http://localhost:5001`**.

---

## ⚙️ Configuration (`service/models.json`)

To add, edit, or reorder available models in the dropdowns, update `llm_comparison/service/models.json`:

```json
[
  {
    "name": "openrouter/free",
    "display_name": "OpenRouter Free Auto-Router",
    "description": "Automatically routes your request to the fastest available free model on OpenRouter."
  },
  {
    "name": "google/gemma-4-26b-a4b-it:free",
    "display_name": "Gemma 4 26B (Google)",
    "description": "26B parameter multimodal model by Google DeepMind."
  }
]
```

---

## 🧪 Running Unit Tests

Run the automated test suite from the repository root:

```bash
python3 -m unittest discover tests
```
