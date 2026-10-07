# AI Village Workshops

To contribute, make some containers and a `docker-compose.yml` that connects them and spins up a website.

1. Aside from LLMs, each workshop has to be a self-contained set of containers.
2. The containers need to serve a single website.
3. GPU memory is strictly limited: cluster worker nodes have single 16GB VRAM GPUs (NVIDIA RTX 2000 Ada). Any challenge model must fit within 16GB (or use quantization).

## Keep in Mind

For 90% of the participants, they will not learn the skill from your workshop. These are for people who drop in and there is a lot of other things going on at hacker cons.

Them not learning the skill is fine; your task is to get them excited to learn and to guide them when they do. Prepare them for learning. You have succeeded if they leave knowing why this is important and wanting to know more.

Instilling a joy for the topic is far more difficult than a technical demo. You need to think about how much they need to invest before they get something out. This initial payoff can be as simple as your UI being pretty. It can also be your personal enthusiasm, but this does not scale.

The best demos and workshops are usually not technically complex. They are accessible, they speak to people respectfully, and they have a deep subject behind them. You studied that deep subject for a reason—show off that joy.

## Architecture & Deployment

This is deployed with a Rust platform that spins up user pods on demand and uses Pingora to reverse-proxy attendees over to their pod, identifying each user with a session cookie.

You have two pods to work with:
- **User Pod (`[user]`)**: Deployed per attendee on demand. This pod serves the user-facing web interface. The per-user pod can hold state directly in the container (for example, in-memory lists or local session files). You should not need a database or extra storage layers.
- **Central Service Pod (`[service]`)**: An optional shared backend container deployed once for the whole workshop. All attendee pods for that workshop can communicate with it over internal cluster DNS at `http://<workshop-name>-service:<port>`. Use this for verification servers, grading logic, or shared backends.

## Workshop Definition (`workshop.toml`)

Each workshop directory contains a `workshop.toml` file at its root to declare its metadata, container images, ports, and environment variables.

### Example `workshop.toml`

```toml
name = "email-indirect"
description = "OmniCorp executive email assistant. Probe the agent and exfiltrate data."
model = "google/gemma-3-4b-it" # Optional: vLLM model to preload

[user]
image = "ghcr.io/aivillage/workshops/email-indirect-user"
port = 5000
launch_uri = "/"                       # Optional: URL path to launch in the browser

[user.env]
CHALLENGE_SERVER_HOST = "email-indirect-service"
CHALLENGE_SERVER_PORT = "2525"

# Optional: Shared central backend service
[service]
image = "ghcr.io/aivillage/workshops/email-indirect-service"
port = 2525

[service.env]
VLLM_URL = "http://vllm-gemma3:8000/v1/chat/completions"
VLLM_MODEL = "gemma-3"
```

### Supported Keys

- **`name`** (required): Unique name of the workshop directory (matches folder name).
- **`description`** (required): Short description displayed on the workshop hub landing page.
- **`model`** (optional): Name of the cluster vLLM model required by this workshop. The cluster preloads and caches this model.
- **`[user]`** (required): The per-attendee container specification.
  - `image`: Container image path.
  - `port`: Port the web application listens on inside the container (default `5000`).
  - `launch_uri`: Optional relative URL path opened when the user clicks launch (e.g. `/?presentation=spatial&kiosk=1`).
  - `extraIngressPorts`: Optional list of additional TCP ports to expose (e.g. `[25]`).
- **`[user.env]`** (optional): Environment variables injected into the user container.
- **`[service]`** (optional): Companion central service container specification.
  - `image`: Container image path.
  - `port`: Internal port the service listens on.
- **`[service.env]`** (optional): Environment variables injected into the service container.

## Container Images & Downstream Manifest Compilation

Each `workshop.toml` is consumed directly as a Nix flake input by downstream event lab repositories (such as `lab1`). During cluster deployment, the lab compiler reads these definitions to generate the cluster's Kubernetes manifests (`workshop-hub.yaml` for dynamic attendee pod spawning and `workshops.yaml` for shared backend services).

### Dynamic Tag Resolution (CI-Built Images)

For containers built and published by this repository's CI/CD pipeline, **omit the tag** from the image path:

```toml
[user]
image = "ghcr.io/aivillage/workshops/prompt-extraction-user" # Omit tag
```

- **CI Publishing**: When changes are merged or pushed to this repository, GitHub Actions builds the container images and publishes them to GitHub Container Registry tagged with the short Git commit SHA (`TAG=$(git rev-parse --short HEAD)`).
- **Consumer Flake Resolution**: The consuming lab repository locks `workshops` as a flake input in `flake.lock`. When Kubernetes manifests are compiled (`cluster gen k8s`), the compiler automatically resolves bare image names by appending the locked Git revision (`:${inputs.workshops.shortRev}`).
- **Cluster Updates**: Updating workshop container versions in the cluster is managed by updating the flake lock (`nix flake update workshops`), ensuring deployed manifests strictly track the exact code revision in the registry.

### Pinned Images (Exceptions)

If a workshop relies on an external image or a static pre-built container not built by this repository's CI/CD workflow (such as `llm-embeddings` or `model-lab`), specify the explicit tag:

```toml
[user]
image = "ghcr.io/aivillage/workshops/llm-embeddings:667af5d"
```

When an explicit tag is present, the downstream compiler preserves it verbatim.

## LLMs

The LLMs are hosted separately on the cluster's GPU nodes. If your challenge uses an LLM, read the endpoint and model name from environment variables:

```python
import os

VLLM_URL = os.environ.get("VLLM_URL", "http://vllm-gemma3:8000/v1/chat/completions")
VLLM_MODEL = os.environ.get("VLLM_MODEL", "gemma-3")
```

For local testing and development, you can point these variables to a locally hosted Ollama or vLLM instance.
