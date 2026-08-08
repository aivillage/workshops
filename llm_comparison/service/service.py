import json
import os
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from flask import Flask, request, jsonify
from common import LLMClient

app = Flask(__name__)

# Load models configuration from models.json in service directory
MODELS_CONFIG_PATH = Path(__file__).parent / "models.json"


def load_models_config():
    if MODELS_CONFIG_PATH.exists():
        with open(MODELS_CONFIG_PATH, "r", encoding="utf-8") as f:
            return json.load(f)
    return [
        {
            "name": "google/gemma-2-9b-it:free",
            "display_name": "Gemma 2 9B (Google)",
            "description": "High-efficiency instruction-tuned model developed by Google DeepMind."
        },
        {
            "name": "meta-llama/llama-3.3-70b-instruct:free",
            "display_name": "Llama 3.3 70B (Meta)",
            "description": "State-of-the-art open weights 70B parameter model by Meta for complex reasoning."
        }
    ]


MODELS_LIST = load_models_config()


def query_single_model(model_name: str, messages: list, timeout: int = 60) -> dict:
    """Execute OpenAI-compatible LLM call for a specific model in parallel."""
    client = LLMClient(model=model_name)
    start_time = time.time()
    try:
        result = client.generate_response(
            messages=messages,
            max_tokens=512,
            temperature=0.7,
            timeout=timeout
        )
        elapsed_ms = int((time.time() - start_time) * 1000)

        # Estimate token sizes based on word/character count if raw tokens aren't provided by API
        prompt_text = " ".join([m.get("content", "") for m in messages])
        prompt_tokens = max(1, len(prompt_text.split()))
        completion_tokens = max(1, len(result.split()))

        return {
            "model_name": model_name,
            "content": result,
            "latency_ms": elapsed_ms,
            "tokens": {
                "prompt": prompt_tokens,
                "completion": completion_tokens,
                "total": prompt_tokens + completion_tokens
            },
            "status": "success"
        }
    except Exception as e:
        elapsed_ms = int((time.time() - start_time) * 1000)
        return {
            "model_name": model_name,
            "content": f"Error querying model ({model_name}): {str(e)}",
            "latency_ms": elapsed_ms,
            "tokens": {"prompt": 0, "completion": 0, "total": 0},
            "status": "error"
        }


@app.route('/compare', methods=['POST'])
def compare():
    data = request.get_json() or {}
    model_a = data.get("model_a", "google/gemma-2-9b-it:free")
    model_b = data.get("model_b", "meta-llama/llama-3.3-70b-instruct:free")
    history = data.get("history", [])
    prompt = data.get("prompt", "")
    system_prompt = data.get("system_prompt", "")

    if not prompt.strip():
        return jsonify({"error": "Prompt cannot be empty"}), 400

    messages = []
    if system_prompt.strip():
        messages.append({"role": "system", "content": system_prompt.strip()})

    for turn in history:
        if turn.get("user"):
            messages.append({"role": "user", "content": turn["user"]})
        if turn.get("assistant"):
            messages.append({"role": "assistant", "content": turn["assistant"]})

    messages.append({"role": "user", "content": prompt.strip()})

    # Execute both model requests concurrently in thread pool
    with ThreadPoolExecutor(max_workers=2) as executor:
        future_a = executor.submit(query_single_model, model_a, messages)
        future_b = executor.submit(query_single_model, model_b, messages)
        res_a = future_a.result()
        res_b = future_b.result()

    return jsonify({
        "model_a": res_a,
        "model_b": res_b,
        "prompt": prompt.strip()
    })


@app.route('/models', methods=['GET'])
def list_models():
    return jsonify({"models": load_models_config()})


@app.route('/health', methods=['GET'])
def health():
    return jsonify({"status": "ok"})


if __name__ == '__main__':
    port = int(os.environ.get("PORT", 5000))
    print(f"[*] Parallel LLM Exploration Service running on port {port}")
    app.run(host='0.0.0.0', port=port)
