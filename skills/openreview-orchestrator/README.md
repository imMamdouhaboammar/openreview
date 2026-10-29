# OpenReview Orchestrator

Master orchestrator for the OpenReview autonomous reviewing and remediation engine.

## Overview
Directs AI agents across all hosts (Claude Code, ChatGPT, Codex, Gemini CLI, Antigravity) on how to coordinate:
- **`openreview` CLI & daemon:** On-demand reviews, webhook event processing, and pre-publish verification gates.
- **`deep-digger` subsystem:** Causal hypothesis testing, oracle validation, and Rust-accelerated `ddmin` delta debugging.
- **`agency-code-reviewer` standards:** 3-tier severity taxonomy (🔴 Blocker, 🟡 Suggestion, 💭 Nit) with drop-in GitHub ````suggestion```` markdown.
- **`poteto-mode` execution:** Boundary discipline (read-only review vs isolated fix), proof-of-verification, subtractive engineering, and zero conversational slop.

## Installation

Install across universal agent environments:
```bash
./skills/openreview-orchestrator/install.sh
```

Or copy directly into your active skills root:
```bash
cp -r skills/openreview-orchestrator ~/.agents/skills/openreview-orchestrator
```

## Running Deterministic Checks
```bash
# Unit & regression tests
python3 -m unittest discover -s skills/openreview-orchestrator/evals -p 'test_orchestrator.py' -v

# Static security scan
python3 scripts/scan_skill_security.py skills/openreview-orchestrator
```
