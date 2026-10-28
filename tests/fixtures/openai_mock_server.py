"""Local-only Chat Completions fixture for a no-secrets GitHub Actions smoke test.

This server is intentionally never exposed outside 127.0.0.1 and cannot
provide real model inference. It verifies that the full Rust agent reaches
its LLM adapter with an authenticated GitHub PR and isolated sandbox.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class MockCompletion(BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path != "/v1/chat/completions":
            self.send_error(404)
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
            body = json.loads(self.rfile.read(length))
            assert body["model"] == "openreview-e2e-mock"
            assert body["messages"][0]["role"] == "system"
            assert isinstance(body.get("tools"), list)
        except (ValueError, KeyError, AssertionError, TypeError):
            self.send_error(422, "invalid agent completion request")
            return

        response = json.dumps(
            {
                "choices": [
                    {"message": {"role": "assistant", "content": "Dry-run review smoke test completed."}}
                ],
                "usage": {"prompt_tokens": 8, "completion_tokens": 6},
            }
        ).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, *_args):
        pass


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", int(sys.argv[1])), MockCompletion).serve_forever()
