import math
import random
from dataclasses import dataclass, asdict
from typing import Dict, Any, List, Tuple

import numpy as np
from fastapi import FastAPI
from fastapi.responses import HTMLResponse, JSONResponse
from fastapi.staticfiles import StaticFiles

from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
from sklearn.pipeline import Pipeline

# ----------------------------
# Game settings (tweak these)
# ----------------------------
MAX_ATTEMPTS = 30
APPROVE_THRESHOLD = 0.35  # approve if P(fraud) < threshold
RANDOM_SEED = 1337

rng = random.Random(RANDOM_SEED)
np.random.seed(RANDOM_SEED)

FEATURES = [
    "amount",
    "account_age_days",
    "is_international",
    "tx_velocity_10m",
    "prior_chargebacks",
    "device_risk",
    "ip_risk",
]

# Bounds for UI sanity + "realistic-ish" values
BOUNDS = {
    "amount": (1, 5000),
    "account_age_days": (0, 3650),
    "is_international": (0, 1),
    "tx_velocity_10m": (0, 50),
    "prior_chargebacks": (0, 10),
    "device_risk": (0, 100),
    "ip_risk": (0, 100),
}

@dataclass
class Transaction:
    amount: float
    account_age_days: float
    is_international: float
    tx_velocity_10m: float
    prior_chargebacks: float
    device_risk: float
    ip_risk: float

    def to_array(self) -> np.ndarray:
        return np.array([[getattr(self, f) for f in FEATURES]], dtype=float)

def clamp_tx(tx: Dict[str, Any]) -> Dict[str, float]:
    out = {}
    for k in FEATURES:
        lo, hi = BOUNDS[k]
        v = float(tx.get(k, (BOUNDS[k][0] + BOUNDS[k][1]) / 2))
        if v < lo: v = lo
        if v > hi: v = hi
        # Keep binary binary
        if k == "is_international":
            v = 1.0 if v >= 0.5 else 0.0
        out[k] = v
    return out

def synth_row() -> Tuple[np.ndarray, int]:
    """
    Generate a synthetic transaction and label it as fraud/not fraud
    using a hidden rule + noise. This creates a learnable dataset.
    """
    amount = rng.uniform(1, 5000)
    age = rng.uniform(0, 3650)
    intl = 1.0 if rng.random() < 0.18 else 0.0
    vel = rng.uniform(0, 50)
    cb = rng.uniform(0, 10)
    dev = rng.uniform(0, 100)
    ip = rng.uniform(0, 100)

    # Hidden "true" risk function (non-linear-ish, but learnable by logreg)
    score = 0.0
    score += (amount / 5000) * 1.2
    score += (vel / 50) * 1.0
    score += (cb / 10) * 1.6
    score += (dev / 100) * 0.9
    score += (ip / 100) * 0.9
    score += intl * 0.6
    score += (1.0 - min(age / 3650, 1.0)) * 0.7  # new accounts riskier

    # Add noise: reality is messy
    score += rng.uniform(-0.25, 0.25)

    # Convert to probability-ish and sample a label
    p_fraud = 1 / (1 + math.exp(-3.0 * (score - 1.4)))
    y = 1 if rng.random() < p_fraud else 0

    x = np.array([amount, age, intl, vel, cb, dev, ip], dtype=float)
    return x, y

def train_model(n: int = 6000) -> Pipeline:
    X = []
    y = []
    for _ in range(n):
        xi, yi = synth_row()
        X.append(xi)
        y.append(yi)
    X = np.vstack(X)
    y = np.array(y)

    model = Pipeline([
        ("scaler", StandardScaler()),
        ("clf", LogisticRegression(max_iter=400, n_jobs=1)),
    ])
    model.fit(X, y)
    return model

model = train_model()

# ----------------------------
# Per-user-pod state (in memory)
# ----------------------------
base_tx = Transaction(
    amount=850,
    account_age_days=12,
    is_international=1,
    tx_velocity_10m=18,
    prior_chargebacks=2,
    device_risk=66,
    ip_risk=72,
)

attempts_used = 0
attempt_history: List[Dict[str, Any]] = []

def predict_fraud_prob(tx_dict: Dict[str, float]) -> float:
    arr = np.array([[tx_dict[f] for f in FEATURES]], dtype=float)
    p = float(model.predict_proba(arr)[0, 1])  # prob of class 1 = fraud
    return p

def score_attempt(candidate: Dict[str, float], base: Dict[str, float]) -> Dict[str, Any]:
    # Normalize distances by feature ranges so score "feels" fair
    d2 = 0.0
    changed = 0
    for f in FEATURES:
        lo, hi = BOUNDS[f]
        denom = (hi - lo) if (hi - lo) != 0 else 1.0
        diff = (candidate[f] - base[f]) / denom
        if abs(candidate[f] - base[f]) > 1e-9:
            changed += 1
        d2 += diff * diff

    l2 = math.sqrt(d2)
    # final score: smaller is better
    final = l2 + 0.07 * changed
    return {"l2": l2, "changed_features": changed, "final_score": final}

# ----------------------------
# FastAPI app
# ----------------------------
app = FastAPI(title="Tabular ML Evasion Challenge")

app.mount("/static", StaticFiles(directory="static"), name="static")

@app.get("/", response_class=HTMLResponse)
def index():
    with open("static/index.html", "r", encoding="utf-8") as f:
        return f.read()

@app.get("/api/state")
def get_state():
    global attempts_used
    base_dict = asdict(base_tx)
    p = predict_fraud_prob(base_dict)
    return {
        "attempts_used": attempts_used,
        "attempts_left": max(0, MAX_ATTEMPTS - attempts_used),
        "approve_threshold": APPROVE_THRESHOLD,
        "base_tx": base_dict,
        "base_p_fraud": p,
        "history": attempt_history[-20:],  # keep UI light
    }

@app.post("/api/attempt")
async def make_attempt(payload: Dict[str, Any]):
    global attempts_used, attempt_history

    if attempts_used >= MAX_ATTEMPTS:
        return JSONResponse({"error": "No attempts left."}, status_code=429)

    tx_in = payload.get("tx", {})
    candidate = clamp_tx(tx_in)
    base_dict = asdict(base_tx)

    p = predict_fraud_prob(candidate)
    approved = p < APPROVE_THRESHOLD
    sc = score_attempt(candidate, base_dict)

    attempts_used += 1
    record = {
        "attempt": attempts_used,
        "p_fraud": p,
        "approved": approved,
        "score": sc,
        "tx": candidate,
    }
    attempt_history.append(record)

    return {
        "attempts_used": attempts_used,
        "attempts_left": max(0, MAX_ATTEMPTS - attempts_used),
        "result": record,
    }

@app.post("/api/reset")
def reset():
    global attempts_used, attempt_history, base_tx
    attempts_used = 0
    attempt_history = []

    # New base transaction each reset (keeps it interesting)
    base_tx = Transaction(
        amount=rng.uniform(50, 1500),
        account_age_days=rng.uniform(0, 60),
        is_international=1.0 if rng.random() < 0.5 else 0.0,
        tx_velocity_10m=rng.uniform(0, 25),
        prior_chargebacks=rng.uniform(0, 4),
        device_risk=rng.uniform(20, 90),
        ip_risk=rng.uniform(20, 90),
    )

    base_dict = asdict(base_tx)
    p = predict_fraud_prob(base_dict)
    return {"ok": True, "base_tx": base_dict, "base_p_fraud": p}
