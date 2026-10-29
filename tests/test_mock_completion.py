"""Integration regressions for the local-only completion fixture."""

import json
import threading
import unittest
from http.server import HTTPServer
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import sys

sys.path.insert(0, str(Path(__file__).parent / "fixtures"))
from openai_mock_server import MockCompletion


class ReadmeRecognitionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = HTTPServer(("127.0.0.1", 0), MockCompletion)
        cls.worker = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.worker.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.worker.join(timeout=2)

    def completion_status(self, readme_content):
        payload = {
            "model": "openreview-e2e-mock",
            "tools": [{"type": "function", "function": {"name": "readFile"}}],
            "messages": [
                {"role": "system", "content": "Read a repository file"},
                {"role": "assistant", "tool_calls": [{"id": "read_repo_readme"}]},
                {"role": "tool", "tool_call_id": "read_repo_readme", "content": readme_content},
            ],
        }
        request = Request(
            f"http://127.0.0.1:{self.server.server_port}/v1/chat/completions",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        try:
            with urlopen(request, timeout=3) as response:
                return response.status
        except HTTPError as error:
            try:
                return error.code
            finally:
                error.close()

    def test_accepts_markdown_heading(self):
        self.assertEqual(self.completion_status("# OpenReview\n"), 200)

    def test_accepts_branded_html_heading(self):
        self.assertEqual(
            self.completion_status('<h1 align="center">OpenReview</h1>\n'),
            200,
        )

    def test_rejects_unrelated_text(self):
        self.assertEqual(
            self.completion_status("This document mentions OpenReview."),
            422,
        )


if __name__ == "__main__":
    unittest.main()
