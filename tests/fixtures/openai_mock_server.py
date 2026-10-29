"""Local-only Chat Completions fixture for a no-secrets GitHub Actions smoke test.

This server is intentionally never exposed outside 127.0.0.1 and cannot
provide real model inference. It verifies that the full Rust agent reaches
its LLM adapter with an authenticated GitHub PR and isolated sandbox.
"""

import json
import re
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


def is_openreview_readme(content):
    """Accept the repo's Markdown or branded HTML title, not an incidental mention."""
    return bool(re.search(
        r"(?m)^# OpenReview[ \t]*$|<h1\b[^>]*>\s*OpenReview\s*</h1>",
        content,
    ))


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

        # A final answer alone would miss broken tool dispatch or history.
        # Require a real two-turn tool cycle through the Rust agent sandbox.
        tool_results = [
            message
            for message in body["messages"]
            if message.get("role") == "tool"
            and message.get("tool_call_id") == "read_repo_readme"
        ]
        if tool_results:
            prior_calls = [
                call
                for message in body["messages"]
                if message.get("role") == "assistant"
                for call in message.get("tool_calls", [])
            ]
            if (
                not any(call.get("id") == "read_repo_readme" for call in prior_calls)
                or not is_openreview_readme(tool_results[0].get("content", ""))
            ):
                self.send_error(422, "tool call history or sandbox output invalid")
                return
            message = {
                "role": "assistant",
                "content": "Read the repository README; review-only tool cycle completed.",
            }
        else:
            if not any(
                tool.get("function", {}).get("name") == "readFile"
                for tool in body["tools"]
            ):
                self.send_error(422, "read-only tool definition unavailable")
                return
            message = {
                "role": "assistant",
                "content": None,
                "tool_calls": [
                    {
                        "id": "read_repo_readme",
                        "type": "function",
                        "function": {
                            "name": "readFile",
                            "arguments": json.dumps({"path": "README.md"}),
                        },
                    }
                ],
            }

        response = json.dumps(
            {
                "choices": [{"message": message}],
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
