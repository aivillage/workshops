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
        services = ["email-indirect", "prompt-extraction", "rag-poisoning"]
        for svc in services:
            svc_dir = repo_root / svc / "service"
            common_dir = svc_dir / "common"
            dockerfile = svc_dir / "Dockerfile"

            # 1. Check common package directory exists in each service folder
            self.assertTrue(common_dir.exists(), f"Missing common package in {common_dir}")
            self.assertTrue((common_dir / "llm_client.py").exists(), f"Missing llm_client.py in {common_dir}")

            # 2. Check Dockerfile includes 'COPY common ./common'
            self.assertTrue(dockerfile.exists(), f"Missing Dockerfile in {svc_dir}")
            dockerfile_content = dockerfile.read_text(encoding="utf-8")
            self.assertIn(
                "COPY common ./common",
                dockerfile_content,
                f"Dockerfile in {svc_dir} missing 'COPY common ./common'",
            )


if __name__ == "__main__":
    unittest.main()
