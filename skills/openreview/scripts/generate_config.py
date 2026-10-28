#!/usr/bin/env python3
"""
generate_config.py - Deterministic Configuration Generator for OpenReview

Scans repository structure to auto-detect language toolchain, recommends
appropriate sandbox and LLM defaults, and emits valid openreview.config.json.
"""

import sys
import os
import json

def detect_ecosystem(base_dir="."):
    if os.path.exists(os.path.join(base_dir, "Cargo.toml")):
        return "rust"
    if os.path.exists(os.path.join(base_dir, "go.mod")):
        return "go"
    if os.path.exists(os.path.join(base_dir, "pyproject.toml")) or os.path.exists(os.path.join(base_dir, "requirements.txt")):
        return "python"
    if os.path.exists(os.path.join(base_dir, "package.json")):
        return "node"
    return "generic"

def generate_config(base_dir=".", provider="anthropic", sandbox="docker", output_path=None):
    ecosystem = detect_ecosystem(base_dir)
    
    ignored_paths = ["dist/**", "build/**", "*.lock", "*.min.js"]
    if ecosystem == "rust":
        ignored_paths.extend(["target/**"])
    elif ecosystem == "go":
        ignored_paths.extend(["bin/**"])
    elif ecosystem == "node":
        ignored_paths.extend(["node_modules/**", ".next/**"])
    elif ecosystem == "python":
        ignored_paths.extend([".venv/**", "__pycache__/**", "*.pyc"])

    config = {
        "version": "1.0.0",
        "bot_username": "openreview",
        "server": {
            "host": "0.0.0.0",
            "port": 3000,
            "webhook_path": "/api/webhooks/github",
            "health_path": "/health",
            "metrics_path": "/metrics"
        },
        "sandbox": {
            "driver": sandbox,
            "docker_image": "ghcr.io/immamdouhaboammar/openreview-sandbox:latest",
            "timeout_seconds": 180,
            "memory_limit_mb": 2048,
            "cpu_limit": 2.0
        },
        "llm": {
            "provider": provider,
            "model": "claude-3-7-sonnet-20250219" if provider == "anthropic" else ("gpt-4o" if provider == "openai" else ("gemini-2.5-flash" if provider == "gemini" else "deepseek-r1:latest")),
            "max_steps": 20,
            "max_tokens": 200000,
            "temperature": 0.2
        },
        "review_rules": {
            "auto_review_on_pr_opened": False,
            "require_bot_mention": True,
            "reaction_approval_triggers_push": True,
            "ignore_draft_prs": True,
            "ignored_paths": ignored_paths
        }
    }

    target = output_path or os.path.join(base_dir, "openreview.config.json")
    with open(target, "w", encoding="utf-8") as f:
        json.dump(config, f, indent=2)

    print(f"Generated {target} (Ecosystem: {ecosystem}, Provider: {provider}, Sandbox: {sandbox})")
    return target

if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "openreview.config.json"
    generate_config(output_path=out)
