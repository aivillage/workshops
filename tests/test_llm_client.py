"""
Unit tests for common/llm_client.py
"""

import os
import json
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock

from common import LLMConfig, LLMClient, load_secrets_env


class TestLLMConfig(unittest.TestCase):

    def setUp(self):
        # Save env state
        self.original_env = os.environ.copy()
        for k in ["VLLM_URL", "VLLM_MODEL", "VLLM_API_KEY", "OPENROUTER_API_KEY"]:
            os.environ.pop(k, None)

    def tearDown(self):
        # Restore env state
        os.environ.clear()
        os.environ.update(self.original_env)

    def test_load_secrets_env(self):
        # Mock file contents
        content = (
            "# Secret env file\n"
            "VLLM_URL=https://openrouter.ai/api/v1/chat/completions\n"
            "VLLM_MODEL=\"google/gemma-2-9b-it:free\"\n"
            "OPENROUTER_API_KEY='sk-or-v1-testkey123'\n"
        )
        with patch("pathlib.Path.exists", return_value=True):
            with patch("builtins.open", unittest.mock.mock_open(read_data=content)):
                config = load_secrets_env(Path("/dummy/path"))
                self.assertEqual(config["VLLM_URL"], "https://openrouter.ai/api/v1/chat/completions")
                self.assertEqual(config["VLLM_MODEL"], "google/gemma-2-9b-it:free")
                self.assertEqual(config["OPENROUTER_API_KEY"], "sk-or-v1-testkey123")

    def test_llm_config_defaults(self):
        with patch("common.llm_client.load_secrets_env", return_value={}):
            cfg = LLMConfig()
            self.assertEqual(cfg.url, "http://llm-services.local/v1/chat/completions")
            self.assertEqual(cfg.model, "vllm-model")
            self.assertEqual(cfg.api_key, "")
            self.assertEqual(cfg.get_headers(), {"Content-Type": "application/json"})

    def test_llm_config_env_vars_and_sanitization(self):
        os.environ["VLLM_URL"] = "https://api.custom.com/v1/chat/completions"
        os.environ["VLLM_MODEL"] = "custom-model-7b"
        os.environ["OPENROUTER_API_KEY"] = '  "sk-or-v1-quotedkey"  '

        with patch("common.llm_client.load_secrets_env", return_value={}):
            cfg = LLMConfig()
            self.assertEqual(cfg.url, "https://api.custom.com/v1/chat/completions")
            self.assertEqual(cfg.model, "custom-model-7b")
            self.assertEqual(cfg.api_key, "sk-or-v1-quotedkey")
            self.assertEqual(
                cfg.get_headers(),
                {
                    "Content-Type": "application/json",
                    "Authorization": "Bearer sk-or-v1-quotedkey",
                },
            )

    def test_llm_config_fallback_to_vllm_api_key(self):
        os.environ["VLLM_API_KEY"] = "sk-vllm-secret"

        with patch("common.llm_client.load_secrets_env", return_value={}):
            cfg = LLMConfig()
            self.assertEqual(cfg.api_key, "sk-vllm-secret")
            self.assertIn("Authorization", cfg.get_headers())


class TestLLMClient(unittest.TestCase):

    def setUp(self):
        self.config = LLMConfig(
            url="https://mock.api/v1/chat/completions",
            model="mock-model",
            api_key="mock-key",
        )
        self.client = LLMClient(self.config)

    @patch("requests.post")
    def test_chat_completion_success(self, mock_post):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [{"message": {"content": "Hello from mock LLM!"}}]
        }
        mock_post.return_value = mock_response

        messages = [{"role": "user", "content": "Hi"}]
        res = self.client.chat_completion(messages)

        self.assertEqual(res["choices"][0]["message"]["content"], "Hello from mock LLM!")
        mock_post.assert_called_once()
        args, kwargs = mock_post.call_args
        self.assertEqual(args[0], "https://mock.api/v1/chat/completions")
        self.assertEqual(kwargs["headers"]["Authorization"], "Bearer mock-key")

    @patch("requests.post")
    def test_tool_calling_400_retry_fallback(self, mock_post):
        # 1st call returns 400, 2nd call returns 200
        resp_400 = MagicMock()
        resp_400.status_code = 400

        resp_200 = MagicMock()
        resp_200.status_code = 200
        resp_200.json.return_value = {
            "choices": [{"message": {"content": "Fallback response without tools"}}]
        }

        mock_post.side_effect = [resp_400, resp_200]

        messages = [{"role": "user", "content": "Check calendar"}]
        tools = [{"type": "function", "function": {"name": "list_events"}}]

        res = self.client.chat_completion(messages, tools=tools)

        self.assertEqual(mock_post.call_count, 2)
        self.assertEqual(res["choices"][0]["message"]["content"], "Fallback response without tools")

    @patch("requests.post")
    def test_generate_response_with_native_tool_call(self, mock_post):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.json.return_value = {
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "function": {
                                    "name": "list_events",
                                    "arguments": '{"date": "2028-10-26"}'
                                }
                            }
                        ]
                    }
                }
            ]
        }
        mock_post.return_value = mock_response

        messages = [{"role": "user", "content": "List meetings for 2028-10-26"}]
        out = self.client.generate_response(messages)

        parsed = json.loads(out)
        self.assertEqual(parsed["tool"], "list_events")
        self.assertEqual(parsed["args"]["date"], "2028-10-26")


if __name__ == "__main__":
    unittest.main()
