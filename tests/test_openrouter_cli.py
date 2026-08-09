#!/usr/bin/env python3
"""
OpenRouter API CLI Test Script
Loads config from ~/.config/aiv-workshops/secrets.env (or environment variables)
and executes a test query to verify API authentication and model responsiveness.
"""

import os
import sys
import json
import argparse
import urllib.request
import urllib.error
from pathlib import Path

from common.llm_client import LLMConfig

DEFAULT_CONFIG_PATH = Path.home() / ".config" / "aiv-workshops" / "secrets.env"

def mask_key(key: str) -> str:
    if not key:
        return "<EMPTY>"
    if len(key) <= 8:
        return "********"
    return f"{key[:7]}...{key[-4:]}"

def main():
    parser = argparse.ArgumentParser(description="Test OpenRouter API configuration from CLI")
    parser.add_argument("--config", type=str, default=str(DEFAULT_CONFIG_PATH), help="Path to env config file")
    parser.add_argument("--prompt", type=str, default="Hello! Confirm you are working and state your model name.", help="Prompt to send")
    parser.add_argument("--model", type=str, default=None, help="Override model name")
    parser.add_argument("--url", type=str, default=None, help="Override API URL")
    parser.add_argument("-d", "--debug", action="store_true", help="Enable debug mode (display all HTTP headers and raw JSON)")
    args = parser.parse_args()

    config_path = Path(args.config)
    llm_cfg = LLMConfig(url=args.url, model=args.model, secrets_file=config_path)

    api_key = llm_cfg.api_key
    vllm_url = llm_cfg.url
    vllm_model = llm_cfg.model

    print("==================================================")
    print("🚀 OpenRouter API Configuration Test")
    print("==================================================")
    print(f"Config File : {config_path} ({'Found' if config_path.exists() else 'Not Found'})")
    print(f"API Endpoint: {vllm_url}")
    print(f"Model Req   : {vllm_model}")
    print(f"API Key     : {mask_key(api_key)}")
    print(f"Debug Mode  : {'Enabled' if args.debug else 'Disabled'}")
    print("--------------------------------------------------")

    if not api_key:
        print("❌ Error: OPENROUTER_API_KEY is empty or missing!")
        print(f"   Please check '{config_path}' or set OPENROUTER_API_KEY in your environment.")
        sys.exit(1)

    payload = {
        "model": vllm_model,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant being tested via CLI."},
            {"role": "user", "content": args.prompt}
        ],
        "temperature": 0.7,
        "max_tokens": 256
    }

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}"
    }

    req = urllib.request.Request(
        vllm_url,
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST"
    )

    print(f"Sending prompt: '{args.prompt}' ...")
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            status_code = resp.getcode()
            response_headers = dict(resp.info())
            raw_bytes = resp.read()
            response_data = json.loads(raw_bytes.decode("utf-8"))

            print("\n==================================================")
            print(f"✅ Success! (HTTP Status {status_code})")
            print("==================================================")

            # Display Router / Model details from response JSON
            routed_model = response_data.get("model", "N/A")
            provider_info = response_data.get("provider", response_headers.get("x-openrouter-provider", "N/A"))
            print(f"Routed Model: {routed_model}")
            if provider_info != "N/A":
                print(f"Provider    : {provider_info}")

            if args.debug:
                print("\n--------------------------------------------------")
                print("📋 [DEBUG] Response HTTP Headers:")
                print("--------------------------------------------------")
                for key, val in response_headers.items():
                    print(f"  {key}: {val}")
                print("--------------------------------------------------")

            choices = response_data.get("choices", [])
            if choices and len(choices) > 0:
                message_content = choices[0].get("message", {}).get("content", "")
                print(f"\nAssistant Response:\n{message_content.strip()}\n")
            else:
                print(f"\nRaw Response JSON:\n{json.dumps(response_data, indent=2)}\n")

            if args.debug and choices:
                print("--------------------------------------------------")
                print("🔍 [DEBUG] Full Response JSON:")
                print("--------------------------------------------------")
                print(json.dumps(response_data, indent=2))
                print("--------------------------------------------------")

            usage = response_data.get("usage")
            if usage:
                print(f"Tokens Used: Prompt={usage.get('prompt_tokens', 0)}, Completion={usage.get('completion_tokens', 0)}, Total={usage.get('total_tokens', 0)}")

    except urllib.error.HTTPError as e:
        print("\n==================================================")
        print(f"❌ HTTP Error {e.code}: {e.reason}")
        print("==================================================")
        
        if args.debug:
            print("\n--------------------------------------------------")
            print("📋 [DEBUG] Error HTTP Headers:")
            print("--------------------------------------------------")
            for key, val in dict(e.headers).items():
                print(f"  {key}: {val}")
            print("--------------------------------------------------")

        try:
            err_body = json.loads(e.read().decode("utf-8"))
            print(f"Error Details:\n{json.dumps(err_body, indent=2)}")
        except Exception:
            print(f"Error Body: {e.read().decode('utf-8', errors='ignore')}")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Network or System Error: {str(e)}")
        sys.exit(1)

if __name__ == "__main__":
    main()
