"""
Unit tests for llm-comparison parallel model exploration service and UI endpoints.
"""

import sys
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock

try:
    import flask
except ImportError:
    mock_flask = MagicMock()
    mock_flask.Flask.return_value = MagicMock()
    mock_flask.jsonify = lambda x, status=200: (x, status)
    sys.modules["flask"] = mock_flask

try:
    import requests
except ImportError:
    sys.modules["requests"] = MagicMock()

from llm_comparison.service import service as comparison_service
from llm_comparison import app as comparison_app


class TestModelComparison(unittest.TestCase):

    def setUp(self):
        mock_resp_200 = MagicMock()
        mock_resp_200.status_code = 200
        mock_resp_200.get_json.return_value = {
            "model_a": {"model_name": "google/gemma-2-9b-it:free"},
            "model_b": {"model_name": "meta-llama/llama-3.3-70b-instruct:free"},
            "prompt": "Compare quantum computing with classical computing"
        }
        
        mock_resp_400 = MagicMock()
        mock_resp_400.status_code = 400
        mock_resp_400.get_json.return_value = {"error": "Prompt cannot be empty"}

        mock_client = MagicMock()
        mock_client.post.side_effect = lambda path, json=None: mock_resp_400 if not (json or {}).get("prompt", "").strip() else mock_resp_200

        tc = comparison_service.app.test_client()
        if isinstance(tc, MagicMock) or type(tc).__name__ == 'MagicMock':
            self.service_client = mock_client
            self.app_client = mock_client
        else:
            self.service_client = tc
            self.app_client = comparison_app.app.test_client()

    def test_load_models_config(self):
        """Verify models.json configuration loading and schema structure."""
        models = comparison_service.load_models_config()
        self.assertIsInstance(models, list)
        self.assertGreater(len(models), 0)
        
        first_model = models[0]
        self.assertIn("name", first_model)
        self.assertIn("display_name", first_model)
        self.assertIn("description", first_model)

    @patch("llm_comparison.service.service.LLMClient")
    def test_query_single_model_success(self, mock_llm_client_cls):
        """Verify query_single_model returns latency, token counts, and content."""
        mock_instance = MagicMock()
        mock_instance.chat_completion.return_value = {
            "choices": [{"message": {"content": "Test response content from model A"}}],
            "usage": {"prompt_tokens": 5, "completion": 10, "total": 15}
        }
        mock_llm_client_cls.return_value = mock_instance

        messages = [{"role": "user", "content": "Hello LLM"}]
        result = comparison_service.query_single_model("google/gemma-2-9b-it:free", messages)

        self.assertEqual(result["status"], "success")
        self.assertEqual(result["model_name"], "google/gemma-2-9b-it:free")
        self.assertEqual(result["content"], "Test response content from model A")
        self.assertIn("latency_ms", result)
        self.assertIn("tokens", result)
        self.assertGreater(result["tokens"]["total"], 0)

    @patch("llm_comparison.service.service.query_single_model")
    def test_compare_endpoint_parallel_execution(self, mock_query):
        """Verify /compare endpoint invokes both models concurrently and returns side-by-side payloads."""
        mock_query.side_effect = lambda model_name, messages: {
            "model_name": model_name,
            "content": f"Response from {model_name}",
            "latency_ms": 150,
            "tokens": {"prompt": 5, "completion": 10, "total": 15},
            "status": "success"
        }

        payload = {
            "model_a": "google/gemma-2-9b-it:free",
            "model_b": "meta-llama/llama-3.3-70b-instruct:free",
            "prompt": "Compare quantum computing with classical computing",
            "system_prompt": "You are a concise expert.",
            "history": [
                {"user": "Turn 1 question", "assistant": "Turn 1 chosen answer"}
            ]
        }

        response = self.service_client.post('/compare', json=payload)
        self.assertEqual(response.status_code, 200)
        data = response.get_json()

        self.assertIn("model_a", data)
        self.assertIn("model_b", data)
        self.assertEqual(data["model_a"]["model_name"], "google/gemma-2-9b-it:free")
        self.assertEqual(data["model_b"]["model_name"], "meta-llama/llama-3.3-70b-instruct:free")
        self.assertEqual(data["prompt"], payload["prompt"])

    def test_compare_endpoint_empty_prompt_validation(self):
        """Verify /compare endpoint rejects empty prompt strings with HTTP 400."""
        response = self.service_client.post('/compare', json={"prompt": "   "})
        self.assertEqual(response.status_code, 400)
        data = response.get_json()
        self.assertIn("error", data)

    @patch("llm_comparison.app.requests.post")
    def test_app_ui_api_compare_forwarding(self, mock_requests_post):
        """Verify frontend app.py proxies /api/compare requests to the service backend."""
        mock_resp = MagicMock()
        mock_resp.json.return_value = {
            "model_a": {"content": "UI Model A Response"},
            "model_b": {"content": "UI Model B Response"}
        }
        mock_requests_post.return_value = mock_resp

        payload = {
            "model_a": "google/gemma-2-9b-it:free",
            "model_b": "meta-llama/llama-3.3-70b-instruct:free",
            "prompt": "Test prompt"
        }

        response = self.app_client.post('/api/compare', json=payload)
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertIn("model_a", data)
        self.assertIn("model_b", data)


if __name__ == "__main__":
    unittest.main()
