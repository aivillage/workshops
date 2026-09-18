#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-}"
CONTEXT="${2:-}"
shift 2 || true

PROJECT_ROOT="${PROJECT_ROOT:-$(
  if [[ -f "$(dirname "${BASH_SOURCE[0]}")/../catalog.json" ]]; then
    cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
  else
    pwd
  fi
)}"
PUSH=false
TAG=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --push)
      PUSH=true
      shift
      ;;
    --tag)
      TAG="$2"
      shift 2
      ;;
    *)
      if [[ -z "$TAG" ]]; then
        TAG="$1"
        shift
      else
        echo "Unknown argument: $1" >&2
        exit 1
      fi
      ;;
  esac
done

if [[ -z "$TAG" ]]; then
  if GIT_SHA=$(git -C "$PROJECT_ROOT" rev-parse --short HEAD 2>/dev/null); then
    TAG="$GIT_SHA"
  else
    TAG="latest"
  fi
fi

if command -v podman &>/dev/null; then
  CONTAINER_CMD="podman"
elif [[ -x "/opt/homebrew/bin/podman" ]]; then
  CONTAINER_CMD="/opt/homebrew/bin/podman"
elif command -v docker &>/dev/null; then
  CONTAINER_CMD="docker"
else
  CONTAINER_CMD="docker"
fi

build_one() {
  local name="$1"
  local path="$2"
  local image="ghcr.io/aivillage/workshops/${name}"
  local full_tag="${image}:${TAG}"

  echo "=== 🔨 Building ${name} (Tag: ${TAG}) ==="
  "$CONTAINER_CMD" build --platform linux/amd64 -t "${full_tag}" "${PROJECT_ROOT}/${path}"

  if [[ "$PUSH" == "true" ]]; then
    echo "=== 🚀 Pushing ${full_tag} ==="
    "$CONTAINER_CMD" push "${full_tag}"

    local current_branch
    current_branch="${GITHUB_REF_NAME:-$(git -C "$PROJECT_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")}"
    if [[ "$current_branch" == "main" ]]; then
      local latest_tag="${image}:latest"
      if [[ "${full_tag}" != "${latest_tag}" ]]; then
        echo "=== Tagging and pushing ${latest_tag} ==="
        "$CONTAINER_CMD" tag "${full_tag}" "${latest_tag}"
        "$CONTAINER_CMD" push "${latest_tag}"
      fi
    fi
  fi
}

if [[ "$TARGET" == "--all" ]]; then
  echo "=== Building all container images ==="
  while read -r c_name c_path; do
    if [[ -n "$c_name" && -n "$c_path" ]]; then
      build_one "$c_name" "$c_path"
    fi
  done < <(python3 -c "import json; [print(c['name'], c['path']) for c in json.load(open('$PROJECT_ROOT/catalog.json'))['containers']]")
  echo "=== Finished building all images ==="
else
  if [[ -z "$TARGET" || -z "$CONTEXT" ]]; then
    echo "Usage: build_container.sh <name> <context_path> [--push] [--tag <tag>]" >&2
    echo "       build_container.sh --all \"\" [--push] [--tag <tag>]" >&2
    exit 1
  fi
  build_one "$TARGET" "$CONTEXT"
fi
