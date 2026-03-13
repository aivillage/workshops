# Who Controls the Algorithm?

An interactive machine learning challenge exploring how automated decision systems can be probed, influenced, and reverse-engineered through controlled inputs.

This project simulates a deployed fraud detection model and allows users to explore how decision boundaries can be discovered — even in black-box settings.

---

## Run the Challenge

### Build from Source

```bash
git clone https://github.com/aivillage/workshops.git
cd tabular-ml-evasion
docker compose up --build
echo "Open http://$(docker compose port web 8080)"
```

Then copy the URL:

```
http://localhost:{port}
```

---

## What This Demonstrates

This challenge models a real-world ML deployment scenario:

* A fraud detection system evaluates a transaction
* You are not told how the model works
* You can modify input features
* You observe the decision output

**Your goal:** Get **APPROVED** with minimal changes.

---

## System Modes

### Transparent Mode

* Displays probability of fraud
* Demonstrates how confidence leakage accelerates exploitation

### Black-Box Mode

* Displays only `APPROVED` or `FRAUD`
* Simulates real-world deployed systems
* Requires players to probe and approximate the decision boundary

---

##  Learning Objectives

Players learn:

* How ML decision boundaries can be probed via input manipulation
* How feature sensitivity can be inferred through experimentation
* Why exposing model confidence increases exploitability
* How constrained optimization affects adversarial strategy
* Why query limits matter in deployed ML systems

> This is not about committing fraud — it is about understanding the fragility and influenceability of automated decision systems.

---

##  Defender Takeaways

This demo highlights why real systems should:

* Avoid exposing raw probability outputs
* Monitor repeated near-boundary probing
* Implement rate limiting
* Detect feature manipulation patterns
* Add behavioral consistency checks
* Consider stochastic defenses

---

##  System Overview

* **Backend:** FastAPI
* **Model:** Scikit-learn (Logistic Regression)
* **Data:** Synthetic transaction generator
* **Containerized:** Docker
* **Deployment:** Runs fully offline
* **External APIs:** None required

---

##  Challenge Rules

* You have a limited query budget
* Your score penalizes:

  * Large feature changes
  * Too many modified features

🏆 **Best Score = Minimal deviation + Approval**

---

## 🎭 Theme: Agency

Automated systems increasingly make decisions about:

* Financial access
* Fraud status
* Insurance eligibility
* Content moderation

When systems decide outcomes — **who has agency?**

This challenge allows users to explore how algorithmic decisions can be influenced — and what that means for transparency, control, and power.