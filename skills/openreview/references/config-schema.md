# OpenReview Configuration Specification (`openreview.config.json`)

This document defines the schema, default values, and operational semantics for configuring OpenReview on any repository.

---

## JSON Schema Definition

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OpenReviewConfig",
  "type": "object",
  "required": ["version", "bot_username", "server", "sandbox", "llm"],
  "properties": {
    "version": {
      "type": "string",
      "enum": ["1.0.0"]
    },
    "bot_username": {
      "type": "string",
      "default": "openreview",
      "description": "GitHub username or bot mention handle (without '@') that triggers on-demand reviews."
    },
    "server": {
      "type": "object",
      "required": ["host", "port"],
      "properties": {
        "host": { "type": "string", "default": "0.0.0.0" },
        "port": { "type": "integer", "minimum": 1, "maximum": 65535, "default": 3000 },
        "webhook_path": { "type": "string", "default": "/api/webhooks/github" },
        "health_path": { "type": "string", "default": "/health" },
        "metrics_path": { "type": "string", "default": "/metrics" }
      }
    },
    "sandbox": {
      "type": "object",
      "required": ["driver"],
      "properties": {
        "driver": {
          "type": "string",
          "enum": ["local", "docker"],
          "default": "docker",
          "description": "Sandbox isolation driver: 'docker' uses isolated OCI containers; 'local' runs in temporary subprocess directories."
        },
        "docker_image": {
          "type": "string",
          "default": "ghcr.io/immamdouhaboammar/openreview-sandbox:latest",
          "description": "Base OCI container image containing compilers, runtimes, and linters."
        },
        "timeout_seconds": {
          "type": "integer",
          "minimum": 10,
          "maximum": 600,
          "default": 180
        },
        "memory_limit_mb": {
          "type": "integer",
          "minimum": 256,
          "maximum": 8192,
          "default": 2048
        },
        "cpu_limit": {
          "type": "number",
          "minimum": 0.5,
          "maximum": 8.0,
          "default": 2.0
        }
      }
    },
    "llm": {
      "type": "object",
      "required": ["provider", "model"],
      "properties": {
        "provider": {
          "type": "string",
          "enum": ["anthropic", "openai", "gemini", "ollama"],
          "default": "anthropic"
        },
        "model": {
          "type": "string",
          "default": "claude-3-7-sonnet-20250219"
        },
        "max_steps": {
          "type": "integer",
          "minimum": 1,
          "maximum": 50,
          "default": 20
        },
        "max_tokens": {
          "type": "integer",
          "minimum": 1000,
          "maximum": 500000,
          "default": 200000
        },
        "temperature": {
          "type": "number",
          "minimum": 0.0,
          "maximum": 1.0,
          "default": 0.2
        }
      }
    },
    "review_rules": {
      "type": "object",
      "properties": {
        "auto_review_on_pr_opened": { "type": "boolean", "default": false },
        "require_bot_mention": { "type": "boolean", "default": true },
        "reaction_approval_triggers_push": { "type": "boolean", "default": true },
        "ignore_draft_prs": { "type": "boolean", "default": true },
        "ignored_paths": {
          "type": "array",
          "items": { "type": "string" },
          "default": ["dist/**", "build/**", "node_modules/**", "target/**", "*.lock", "*.min.js"]
        }
      }
    }
  },
  "additionalProperties": false
}
```

---

## Canonical `.env.example` Specification

```bash
# =================================================================
# OpenReview Server & Daemon Settings
# =================================================================
HOST=0.0.0.0
PORT=3000
BOT_USERNAME=openreview

# =================================================================
# GitHub App Authentication (Mandatory for Webhook Mode)
# =================================================================
GITHUB_APP_ID=123456
GITHUB_APP_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA..."
GITHUB_APP_WEBHOOK_SECRET=your_secure_hmac_secret_token
GITHUB_APP_INSTALLATION_ID=98765432

# =================================================================
# LLM Provider Configuration
# Choose: anthropic | openai | gemini | ollama
# =================================================================
LLM_PROVIDER=anthropic
LLM_MODEL=claude-3-7-sonnet-20250219
ANTHROPIC_API_KEY=sk-ant-api03-...

# Alternative Providers:
# OPENAI_API_KEY=sk-...
# OPENAI_BASE_URL=https://api.openai.com/v1
# GEMINI_API_KEY=AIzaSy...
# OLLAMA_BASE_URL=http://localhost:11434

# =================================================================
# Sandbox Isolation Driver ("docker" | "local")
# =================================================================
SANDBOX_DRIVER=docker
```
