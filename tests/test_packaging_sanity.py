"""
Sanity and packaging tests for workshop services and Dockerfiles.
"""

import unittest
from pathlib import Path

repo_root = Path(__file__).resolve().parent.parent


class TestPackagingSanity(unittest.TestCase):

    def test_service_common_module_and_dockerfile_packaging(self):
        """
        Verify that all workshop service folders contain the common module
        and their Dockerfiles include 'COPY common ./common' to prevent
        ModuleNotFoundError inside containers.
        """
        services = ["email-indirect", "prompt-extraction", "rag-poisoning", "llm_comparison"]
        root_common = repo_root / "common"
        self.assertTrue(root_common.exists(), f"Missing root common package in {root_common}")
        self.assertTrue((root_common / "llm_client.py").exists(), "Missing llm_client.py in root common package")

        for svc in services:
            svc_dir = repo_root / svc / "service"
            dockerfile = svc_dir / "Dockerfile"
            self.assertTrue(dockerfile.exists(), f"Missing Dockerfile in {svc_dir}")


if __name__ == "__main__":
    unittest.main()
