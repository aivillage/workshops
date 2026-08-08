# AIV Workshops

To contribute make some containers and a "docker-compose.yml" that connects them and spins up a website.

1. Aside from LLMs, each workshop has to be a self contained set of containers.
2. The containers need to serve a single website.
3. GPU memory access is limited as we only have 96GB of vram in the portable cluster.

## Keep in Mind

For 90% of the participants, they will not learn the skill from your workshop. These are for people who drop in and there's a lot of other things going on at hacker cons. 

Them not learning the skill is *fine*, your task is to get them excited to learn and to guide them when they do. Prepare them for learning. You have succeeded if they leave knowing why this is important and wanting to know more.

Instilling a joy for the topic is far more difficult than a technical demo. You need to think about how much they need to invest before they get something out. This initial payoff can be as simple as your UI being pretty. It can also be your personal enthusiasm, but this doesn't "scale". 

The best demos/workshops are usually not technically interesting. They are accessible, they speak to people respectfully and they have a deep subject behind them. You studied that deep subject for a reason, show off that joy. 

## Deployment

This is deployed with a rust system that spins up the user pod per user and then uses pingora to proxy them over to their pod, identifying the user with a cookie. It is not designed to be secure, it's designed to deliver them your container with the least friction possible. 

You have **2 pods** to work with a user pod that is deployed per user, and a central pod that all users share. The per-user pod can hold state in the container (see the email-indirect user container that uses a python list for the "emails"). You should not need a database or other layers in your app. Claude/Gemini is very good at vibing front end demos with self contained state. 

## LLMs

The LLMs need to be hosted separately and we can only host a limited number of them. If you need one you should depend on 2 environment variables:

```
VLLM_URL = os.environ.get("VLLM_URL", "http://llm-services.local/v1/")
VLLM_MODEL = os.environ.get("VLLM_MODEL", "vllm-model")
```

We'll add a per-user API key for management and budget if users end up abusing their access. For testing/developement you should point the URL to a locally hosted ollama or vllm.  

---

## 🛠️ Workshop Catalogue

| Workshop | Location | Description | Requirements |
| :--- | :--- | :--- | :--- |
| **Indirect Email Injection** | [`email-indirect/`](./email-indirect) | Indirect prompt injection & LLM function-calling calendar exploit | LLM Endpoint / OpenRouter / Ollama |
| **System Prompt Extraction** | [`prompt-extraction/`](./prompt-extraction) | System prompt exfiltration targeting OmniCorp authorization code | LLM Endpoint / OpenRouter / Ollama |
| **RAG Poisoning** | [`rag-poisoning/`](./rag-poisoning) | Knowledge base injection to manipulate RAG vendor recommendations | LLM Endpoint / OpenRouter / Ollama |
| **Parallel LLM Exploration** | [`llm_comparison/`](./llm_comparison) | Side-by-side multi-turn comparison & context branching across two LLMs | LLM Endpoint / OpenRouter / Ollama |
| **YOLO Adversarial Attack** | [`yolo-l2/`](./yolo-l2) | $L_2$-bounded image perturbation attack against YOLOv11 classification | Offline (PyTorch included) |
| **LLM Embeddings Explorer** | [`llm-embeddings/`](./llm-embeddings) | Live visualization of hidden states, token logits, and FFT spectral density | Offline (`gemma-270m` included) |

---

## 🔐 Local Credentials & Secrets Setup

To test with external LLM APIs (like OpenRouter or Google AI Studio) without committing secrets, create a file outside the workspace at `~/.config/aiv-workshops/secrets.env`:

```env
OPENROUTER_API_KEY=sk-or-v1-your-actual-api-key-without-quotes
VLLM_URL=https://openrouter.ai/api/v1/chat/completions
VLLM_MODEL=google/gemma-2-9b-it:free
```

### Supported API Providers:
* **OpenRouter**: `https://openrouter.ai/api/v1/chat/completions`
* **Google AI Studio (Free Gemini API)**: `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions`
* **Google Cloud Vertex AI**: `https://${REGION}-aiplatform.googleapis.com/v1beta1/projects/${PROJECT_ID}/locations/${REGION}/endpoints/openapi/chat/completions`
* **Local Ollama**: `http://host.docker.internal:11434/v1/chat/completions`

---

## 🧪 Testing API Setup (CLI Utility)

Verify your API configuration and inspect model routing before launching workshops:

```bash
# Standard test
python3 -m tests.test_openrouter_cli

# Debug mode (displays response HTTP headers & raw JSON payload)
python3 -m tests.test_openrouter_cli -d
```

---

## 🚀 Running Workshops Locally

### Method 1: Using Docker Compose
Run inside any workshop directory:
```bash
cd email-indirect
docker-compose --env-file ~/.config/aiv-workshops/secrets.env up --build
```
Access the web UI at `http://localhost:30050` (or `http://penguin.linux.test:30050` on ChromeOS).

### Method 2: Running Directly with Python (No Docker Required)

1. **Install Dependencies**:
   ```bash
   pip install flask aiosmtpd requests
   ```

2. **Terminal 1: Start Central Service**:
   ```bash
   cd email-indirect/service
   export LISTEN_PORT=2525
   export REPLY_PORT=2500
   python3 service.py
   ```

3. **Terminal 2: Start User Web UI**:
   ```bash
   cd email-indirect/user
   export CHALLENGE_SERVER_HOST="127.0.0.1"
   export CHALLENGE_SERVER_PORT=2525
   export MY_LISTEN_PORT=2500
   export WEB_PORT=5000
   python3 client.py
   ```

4. Open browser at `http://localhost:5000`.

---

## 🧩 Shared Common Library & Unit Tests

Shared LLM configuration and client routines are located in [`common/`](./common).

Run the automated test suite with:
```bash
python3 -m unittest discover tests
```
