import json
import os
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from flask import Flask, request, jsonify
from common import LLMConfig, LLMClient

app = Flask(__name__)

# Load models configuration from models.json in service directory
MODELS_CONFIG_PATH = Path(__file__).parent / "models.json"


def load_models_config():
    if MODELS_CONFIG_PATH.exists():
        with open(MODELS_CONFIG_PATH, "r", encoding="utf-8") as f:
            return json.load(f)
    return []


MODELS_LIST = load_models_config()


def query_single_model(model_name: str, messages: list, timeout: int = 60) -> dict:
    """Execute OpenAI-compatible LLM call for a specific model in parallel."""
    config = LLMConfig(model=model_name)
    client = LLMClient(config=config)
    start_time = time.time()
    try:
        raw_resp = client.chat_completion(
            messages=messages,
            max_tokens=512,
            temperature=0.7,
            timeout=timeout
        )
        elapsed_ms = int((time.time() - start_time) * 1000)

        # Extract message content
        content = ""
        if "choices" in raw_resp and len(raw_resp["choices"]) > 0:
            content = raw_resp["choices"][0].get("message", {}).get("content", "")

        # Extract token usage metadata from API or estimate if missing
        usage = raw_resp.get("usage") or {}
        if usage and "total_tokens" in usage:
            tokens = {
                "prompt": usage.get("prompt_tokens", 0),
                "completion": usage.get("completion_tokens", 0),
                "total": usage.get("total_tokens", 0)
            }
        else:
            prompt_text = " ".join([m.get("content", "") for m in messages])
            prompt_tokens = max(1, len(prompt_text.split()))
            completion_tokens = max(1, len(content.split()))
            tokens = {
                "prompt": prompt_tokens,
                "completion": completion_tokens,
                "total": prompt_tokens + completion_tokens
            }

        routed_model = raw_resp.get("model", model_name)
        provider = raw_resp.get("provider", "OpenRouter")

        if not content and "error" in raw_resp:
            err_msg = raw_resp["error"].get("message", "API Error")
            return {
                "model_name": model_name,
                "routed_model": model_name,
                "provider": provider,
                "content": f"Error: {err_msg}",
                "latency_ms": elapsed_ms,
                "tokens": {"prompt": 0, "completion": 0, "total": 0},
                "status": "error"
            }

        return {
            "model_name": model_name,
            "routed_model": routed_model,
            "provider": provider,
            "content": content,
            "latency_ms": elapsed_ms,
            "tokens": tokens,
            "status": "success"
        }
    except Exception as e:
        elapsed_ms = int((time.time() - start_time) * 1000)
        return {
            "model_name": model_name,
            "routed_model": model_name,
            "provider": "N/A",
            "content": f"Error querying model ({model_name}): {str(e)}",
            "latency_ms": elapsed_ms,
            "tokens": {"prompt": 0, "completion": 0, "total": 0},
            "status": "error"
        }


@app.route('/compare', methods=['POST'])
def compare():
    data = request.get_json() or {}
    model_a = data.get("model_a", "openrouter/free")
    model_b = data.get("model_b", "google/gemma-4-31b-it:free")
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


@app.route('/compare_single', methods=['POST'])
def compare_single():
    data = request.get_json() or {}
    model_name = data.get("model_name", "openrouter/free")
    messages = data.get("messages", [])

    res = query_single_model(model_name, messages)
    return jsonify(res)


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
