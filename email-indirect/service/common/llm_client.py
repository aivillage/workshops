"""
Common LLM Client & Configuration Library for AIV Workshops.

Handles configuration loading (VLLM_URL, VLLM_MODEL, VLLM_API_KEY, OPENROUTER_API_KEY),
sanitization of authorization headers, secrets.env fallback loading, and unified 
OpenAI-compatible chat completion requests with automatic tool-calling retry fallbacks.
"""

import os
import json
import requests
from pathlib import Path
from typing import Dict, Any, List, Optional


def load_secrets_env(filepath: Optional[Path] = None) -> Dict[str, str]:
    """Load key-value pairs from secrets.env file if present."""
    if filepath is None:
        filepath = Path.home() / ".config" / "aiv-workshops" / "secrets.env"
    
    config = {}
    if not filepath.exists():
        return config

    try:
        with open(filepath, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" in line:
                    key, val = line.split("=", 1)
                    key = key.strip()
                    val = val.strip().strip('"').strip("'")
                    config[key] = val
    except Exception:
        pass
    return config


class LLMConfig:
    """Manages LLM API endpoints, model selection, and authentication credentials."""

    def __init__(
        self,
        url: Optional[str] = None,
        model: Optional[str] = None,
        api_key: Optional[str] = None,
        secrets_file: Optional[Path] = None,
    ):
        file_config = load_secrets_env(secrets_file)

        # Precedence: Explicit Arg > Env Var > secrets.env > Default
        self.url = (
            url
            or os.environ.get("VLLM_URL")
            or file_config.get("VLLM_URL")
            or "http://llm-services.local/v1/chat/completions"
        )
        self.model = (
            model
            or os.environ.get("VLLM_MODEL")
            or file_config.get("VLLM_MODEL")
            or "vllm-model"
        )

        raw_key = (
            api_key
            or os.environ.get("VLLM_API_KEY")
            or os.environ.get("OPENROUTER_API_KEY")
            or file_config.get("VLLM_API_KEY")
            or file_config.get("OPENROUTER_API_KEY")
            or ""
        )
        self.api_key = raw_key.strip().strip('"').strip("'")

    def get_headers(self) -> Dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers


class LLMClient:
    """Unified client for invoking OpenAI-compatible chat completion APIs."""

    def __init__(self, config: Optional[LLMConfig] = None):
        self.config = config or LLMConfig()

    def chat_completion(
        self,
        messages: List[Dict[str, str]],
        tools: Optional[List[Dict[str, Any]]] = None,
        tool_choice: Optional[str] = None,
        max_tokens: int = 512,
        temperature: float = 0.7,
        timeout: int = 60,
    ) -> Dict[str, Any]:
        """
        Send chat completion request to the configured LLM endpoint.
        Retries without tools if HTTP 400 is returned (tool-calling incompatibility).
        """
        payload: Dict[str, Any] = {
            "model": self.config.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        }
        if tools:
            payload["tools"] = tools
            payload["tool_choice"] = tool_choice or "auto"

        headers = self.config.get_headers()

        resp = requests.post(self.config.url, json=payload, headers=headers, timeout=timeout)

        # Fallback if vLLM/OpenRouter returns 400 (tool-calling incompatibility)
        if resp.status_code == 400 and "tools" in payload:
            print("[!] LLM returned HTTP 400 with tools. Retrying without tools...")
            payload.pop("tools", None)
            payload.pop("tool_choice", None)
            resp = requests.post(self.config.url, json=payload, headers=headers, timeout=timeout)

        resp.raise_for_status()
        return resp.json()

    def generate_response(
        self,
        messages: List[Dict[str, str]],
        tools: Optional[List[Dict[str, Any]]] = None,
        tool_choice: Optional[str] = None,
        max_tokens: int = 512,
        temperature: float = 0.7,
        timeout: int = 60,
    ) -> str:
        """
        High-level generator returning response string or JSON tool-call string.
        """
        resp_json = self.chat_completion(
            messages=messages,
            tools=tools,
            tool_choice=tool_choice,
            max_tokens=max_tokens,
            temperature=temperature,
            timeout=timeout,
        )

        if "choices" in resp_json and len(resp_json["choices"]) > 0:
            choice = resp_json["choices"][0]
            message = choice.get("message", {})

            # Handle native tool calls by returning formatted JSON string
            if "tool_calls" in message and message["tool_calls"]:
                tool_call = message["tool_calls"][0]
                func_name = tool_call["function"]["name"]
                try:
                    func_args = json.loads(tool_call["function"]["arguments"])
                except Exception:
                    func_args = {}
                return json.dumps({"tool": func_name, "args": func_args})

            return message.get("content", "")
        return ""
