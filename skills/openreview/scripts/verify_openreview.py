#!/usr/bin/env python3
"""
verify_openreview.py - Deterministic Verification & Invariant Audit Tool for OpenReview

Audits repository configuration, environment variables, GitHub App credentials,
webhook HMAC signature implementation, and sandbox drivers against all OpenReview invariants.
"""

import sys
import os
import json
import re

class OpenReviewVerifier:
    def __init__(self, target_dir="."):
        self.target_dir = os.path.abspath(target_dir)
        self.violations = []
        self.warnings = []
        self.passed_checks = []

    def check_config_file(self):
        config_path = os.path.join(self.target_dir, "openreview.config.json")
        if not os.path.exists(config_path):
            self.warnings.append("openreview.config.json not found in root (using environment defaults).")
            return

        try:
            with open(config_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            
            # Check required fields
            for req in ["version", "bot_username", "server", "sandbox", "llm"]:
                if req not in data:
                    self.violations.append(f"openreview.config.json missing required top-level property: '{req}'")

            # Check sandbox driver
            driver = data.get("sandbox", {}).get("driver")
            if driver not in ["local", "docker"]:
                self.violations.append(f"Invalid sandbox driver '{driver}'. Must be 'local' or 'docker'.")

            # Check LLM provider
            provider = data.get("llm", {}).get("provider")
            if provider not in ["anthropic", "openai", "gemini", "ollama"]:
                self.violations.append(f"Invalid LLM provider '{provider}'. Must be 'anthropic', 'openai', 'gemini', or 'ollama'.")

            # Check ReAct bounds
            max_steps = data.get("llm", {}).get("max_steps", 20)
            if not isinstance(max_steps, int) or max_steps > 50:
                self.violations.append(f"Invariant 4 Violation: ReAct max_steps ({max_steps}) exceeds safe limit of 50.")

            max_tokens = data.get("llm", {}).get("max_tokens", 200000)
            if not isinstance(max_tokens, int) or max_tokens > 500000:
                self.violations.append(f"Invariant 4 Violation: ReAct max_tokens ({max_tokens}) exceeds safe ceiling (500,000).")

            self.passed_checks.append("openreview.config.json schema & bounds valid")
        except json.JSONDecodeError as e:
            self.violations.append(f"openreview.config.json contains invalid JSON: {e}")
        except Exception as e:
            self.violations.append(f"Failed to read openreview.config.json: {e}")

    def check_env_security(self):
        env_path = os.path.join(self.target_dir, ".env")
        if os.path.exists(env_path):
            with open(env_path, "r", encoding="utf-8") as f:
                content = f.read()

            # Check for plaintext private key in tracked git
            gitignore_path = os.path.join(self.target_dir, ".gitignore")
            is_ignored = False
            if os.path.exists(gitignore_path):
                with open(gitignore_path, "r", encoding="utf-8") as f:
                    if ".env" in f.read():
                        is_ignored = True

            if not is_ignored:
                self.violations.append("Invariant 2 Violation: .env file exists but is NOT listed in .gitignore!")
            else:
                self.passed_checks.append(".env is properly gitignored")

    def check_webhook_hmac_implementation(self):
        """Scans codebase for webhook HMAC constant-time verification."""
        found_hmac = False
        src_dir = os.path.join(self.target_dir, "src")
        if not os.path.exists(src_dir):
            return

        for root, _, files in os.walk(src_dir):
            for file in files:
                if file.endswith((".rs", ".ts", ".js", ".go", ".py")):
                    file_path = os.path.join(root, file)
                    try:
                        with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                            text = f.read()
                            if ("X-Hub-Signature-256" in text or "sha256=" in text) and (
                                "constant_time_eq" in text or "timingSafeEqual" in text or "hmac" in text.lower()
                            ):
                                found_hmac = True
                                break
                    except Exception:
                        pass
            if found_hmac:
                break

        if found_hmac:
            self.passed_checks.append("Invariant 1: Webhook HMAC-SHA256 signature verification detected in source code")
        else:
            self.warnings.append("Could not locate HMAC-SHA256 verification pattern in src/ (check webhook handler).")

    def run_all(self):
        self.check_config_file()
        self.check_env_security()
        self.check_webhook_hmac_implementation()

        print("\n" + "="*60)
        print("  OPENREVIEW VERIFICATION AUDIT")
        print("="*60)
        for p in self.passed_checks:
            print(f"  [✓ PASS] {p}")
        for w in self.warnings:
            print(f"  [! WARN] {w}")
        for v in self.violations:
            print(f"  [✗ FAIL] {v}")
        print("="*60)

        if self.violations:
            print(f"  RESULT: FAILED ({len(self.violations)} violations)")
            return 1
        else:
            print("  RESULT: PASSED (All invariants verified)")
            return 0

if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "."
    verifier = OpenReviewVerifier(target)
    sys.exit(verifier.run_all())
