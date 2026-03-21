"""
LLM Internals Viewer — Flask backend
Serves static frontend + /api/analyze for live-as-you-type analysis.

ENV VARS:
    MODEL_PATH  — path to model dir on disk (default: /models/gemma-270m)
    PORT        — server port (default: 8080)
"""

import os
import numpy as np
import torch
from flask import Flask, request, jsonify, send_from_directory
from transformers import AutoTokenizer, AutoModelForCausalLM

MODEL_PATH = os.environ.get("MODEL_PATH", "/models/gemma-270m")
PORT = int(os.environ.get("PORT", "8080"))
DEVICE = "cuda" if torch.cuda.is_available() else "cpu"
DTYPE = torch.float16 if DEVICE == "cuda" else torch.float32

print(f"Loading model from {MODEL_PATH} on {DEVICE} ({DTYPE}) ...")
tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH, local_files_only=True)
model = AutoModelForCausalLM.from_pretrained(
    MODEL_PATH, local_files_only=True,
    torch_dtype=DTYPE, device_map=DEVICE,
)
model.eval()

input_embed_weight = model.model.embed_tokens.weight
output_embed_weight = model.lm_head.weight
VOCAB_SIZE = int(input_embed_weight.shape[0])
HIDDEN_DIM = int(input_embed_weight.shape[1])
print(f"Model loaded — vocab {VOCAB_SIZE}, dim {HIDDEN_DIM}, device {DEVICE}")

app = Flask(__name__, static_folder="static", static_url_path="")


@app.route("/")
def index():
    return send_from_directory("static", "index.html")


def fft_top_k_matrix(matrix: np.ndarray, k: int = 20):
    """FFT each row of (N, hidden_dim), return (N, k) magnitudes + freq indices."""
    spectra = np.fft.rfft(matrix, axis=1)
    mags = np.abs(spectra)
    # Pick top-k frequencies by mean magnitude across all tokens
    mean_mags = mags.mean(axis=0)
    top_freqs = np.argsort(mean_mags)[::-1][:k].tolist()
    top_freqs.sort()
    # Extract those columns: (N, k)
    reduced = mags[:, top_freqs]
    return {
        "freqs": top_freqs,
        "values": reduced.round(6).tolist(),
    }


@app.route("/api/health", methods=["GET"])
def health():
    return jsonify({
        "status": "ok",
        "model_path": MODEL_PATH,
        "device": DEVICE,
        "vocab_size": VOCAB_SIZE,
        "hidden_dim": HIDDEN_DIM,
    })


@app.route("/api/analyze", methods=["POST"])
def analyze():
    data = request.get_json(silent=True) or {}
    text = data.get("text", "").strip()
    if not text:
        return jsonify({"error": "empty text"}), 400
    if len(text) > 2000:
        return jsonify({"error": "text too long (max 2000 chars)"}), 400

    # 1 — Tokenize
    enc = tokenizer(text, return_tensors="pt", add_special_tokens=False)
    ids = enc["input_ids"][0].tolist()
    tokens = [{"text": tokenizer.decode([tid]), "id": tid} for tid in ids]

    # 2 — Gather embedding matrices (N x hidden_dim)
    with torch.no_grad():
        input_embs = input_embed_weight[ids].float().cpu().numpy()   # (N, H)
        lmhead_embs = output_embed_weight[ids].float().cpu().numpy() # (N, H)

    # 3 — Forward pass: hidden states + logits
    with torch.no_grad():
        out = model(enc["input_ids"].to(DEVICE), output_hidden_states=True)
        output_hidden = out.hidden_states[-1][0].float().cpu().numpy()  # (N, H)

        last_logits = out.logits[0, -1, :].float().cpu()
        probs = torch.softmax(last_logits, dim=-1)
        top_probs, top_ids = torch.topk(probs, 20)
        logits_list = [
            {
                "token": tokenizer.decode([int(tid)]),
                "token_id": int(tid),
                "prob": round(float(p), 6),
                "logit": round(float(last_logits[int(tid)]), 4),
            }
            for p, tid in zip(top_probs, top_ids)
        ]

    # 4 — FFT reduce: (N, hidden_dim) → (N, 20)
    fft_input = fft_top_k_matrix(input_embs, 20)
    fft_output_hidden = fft_top_k_matrix(output_hidden, 20)
    fft_lmhead = fft_top_k_matrix(lmhead_embs, 20)

    return jsonify({
        "tokens": tokens,
        "fft": {
            "input_embed": fft_input,
            "output_hidden": fft_output_hidden,
            "lm_head": fft_lmhead,
        },
        "logits": logits_list,
        "model_path": MODEL_PATH,
        "device": DEVICE,
        "vocab_size": VOCAB_SIZE,
        "hidden_dim": HIDDEN_DIM,
    })


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=PORT, debug=False) 