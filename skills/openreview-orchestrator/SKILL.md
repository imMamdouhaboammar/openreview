---
name: openreview-orchestrator
description: >
  Master orchestrator for the OpenReview autonomous reviewing and remediation engine.
  Directs AI agents on how to execute end-to-end pull request reviews, verified fixes,
  causal regression investigations, and tool coordination across OpenReview CLI,
  deep-digger minimizers, and agency review standards. Enforces Poteto Mode principles
  (boundary discipline, proof-of-verification, subtractive engineering) and 3-tier severity
  classification (Blocker, Suggestion, Nit).
compatibility: ">=1.0.0"
metadata:
  version: "1.0.0"
  author: "Mamdouh Aboammar"
  license: "MIT"
---

# OpenReview Orchestrator: Master System Specification

The OpenReview Orchestrator coordinates the OpenReview system, its auxiliary skills, binary engines, and sandbox boundaries into an authoritative, fail-closed reviewing pipeline.

The canonical execution invariant is:

$$\text{PR Trigger (CLI / Event)} \xrightarrow{\text{Intent Classification}} \text{Execution DAG} \xrightarrow{\text{Sandbox Boundary}} \begin{cases} \text{Read-Only Review} \to \text{Line-Anchored Suggestions} \\ \text{Fix Workspace} \to \text{Verification Gate} \to \text{Attributable Commit} \\ \text{Causal Regression} \to \text{ddmin Minimization} \to \text{Root-Cause Ledger} \end{cases}$$

---

## 1. Operating Modes & System DAG

The orchestrator classifies incoming user requests and PR events into four distinct modes:

| Mode | Trigger Phrases | Engine / Skill Activated | Sandbox Boundary | Output Artifact |
| :--- | :--- | :--- | :--- | :--- |
| **REVIEW** | "review PR", "audit changes", "check for bugs", "openreview review" | `openreview review`, `agency-code-reviewer` | Read-only workspace (no shell write) | Structured review with 3-tier findings & ````suggestion```` blocks |
| **FIX** | "fix issue", "remediate PR", "@openreview fix", "resolve test failure" | `openreview fix`, Pre-publish verification gate | Writable ephemeral sandbox | Verified git commit pushed to PR branch |
| **INVESTIGATE** | "why did this fail", "isolate regression", "minimize repro", "find root cause" | `deep-digger` (`ddmin`, `deep-digger-min`) | Bounded oracle execution | 1-minimal test case & causal hypothesis ledger |
| **VERIFY_CONFIG** | "setup openreview", "verify config", "audit invariants", "check daemon" | `openreview verify-config`, `skills/openreview` | Host configuration inspection | Invariant audit report (`verify_openreview.py`) |

---

## 2. Core Principles Integration (Poteto Mode)

Every action within the orchestrator must satisfy the non-negotiable principles of Poteto Mode:

1. **Boundary Discipline (`principle-boundary-discipline`):**
   - In **Review Mode**, file-write and arbitrary shell tools are strictly disabled. The engine operates exclusively on the authoritative Git diff and read-only file queries.
   - In **Fix Mode**, writes are constrained to an ephemeral sandbox. No host credentials or GitHub App private keys are mounted into the container.
2. **Prove It Works (`principle-prove-it-works`):**
   - An agent must never declare a fix completed or push a branch without deterministic verification evidence.
   - OpenReview's pre-publish verification gate executes the project's native test runner (Cargo, Bun, Pytest, Go, NPM). An exit code of `0` is insufficient unless accompanied by affirmative runner output proving test execution.
3. **Model the Domain (`principle-model-the-domain`):**
   - Encode PR defects into a structured domain taxonomy rather than generic commentary. Every finding belongs to one of three strictly defined severity tiers.
4. **Laziness Protocol & Subtractive Engineering (`principle-laziness-protocol`, `principle-subtract-before-you-add`):**
   - Fixes must be the smallest possible delta that restores correctness. Never introduce unsolicited architectural rewrites, formatting churn, or extraneous dependencies.
5. **Causal Root Causes (`principle-fix-root-causes`):**
   - When a PR breaks tests or exhibits flaky behavior, do not guess fixes. Hand off execution to `deep-digger` to establish a falsifiable hypothesis and reduce reproducing inputs via `deep-digger-min`.
6. **Zero Fluff & Anti-Slop (`principle-minimize-reader-load`, unslop):**
   - Omit conversational preambles ("I reviewed your code and..."), condescending praise ("Good job on this function!"), or empty sign-offs. Present findings immediately under domain headers.

---

## 3. Review Taxonomy & Feedback Standards (`agency-code-reviewer`)

Every review comment must follow the 3-tier severity taxonomy and line-referenced formatting:

### Severity Tiers

- 🔴 **Blocker (Must Fix Before Merge):**
  - Security vulnerabilities (CWE, OWASP Top 10, SQLi, command injection, path traversal, auth bypass).
  - Memory corruption, data loss, race conditions, or unhandled panic/crash paths.
  - Breaking public API contracts without deprecation cycles.
  - Broken compilation or failing test suites.
- 🟡 **Suggestion (Should Fix):**
  - Missing input validation or boundary guards.
  - Performance bottlenecks (N+1 queries, unindexed queries, accidental O(N^2) loops).
  - Insufficient unit test coverage for new edge cases.
  - Fragile error handling or hidden failure modes.
- 💭 **Nit (Nice to Have):**
  - Minor naming clarity improvements.
  - Missing docstrings for complex internal algorithms.
  - Stylistic inconsistencies not caught by automated linters.

### Review Comment Format Rule

```markdown
🔴 **Security: SQL Injection Risk**
Line 42 in `src/db/users.rs`: Raw string concatenation used in query construction.

**Why:** An untrusted caller parameter can escape the query boundary and execute arbitrary SQL commands.

**Suggestion:**
```suggestion
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(&self.pool)
        .await?;
```
```

---

## 4. Engine & CLI Command Matrix

The orchestrator instructs agents to use the installed binaries and scripts directly:

### A. Local Standalone Review (`openreview`)
```bash
# Run read-only review on PR #42 with dry-run protection
GITHUB_TOKEN="$TOKEN" \
LLM_PROVIDER="openai" \
OPENAI_API_KEY="$KEY" \
SANDBOX_DRIVER="local" \
openreview review --repo owner/repo --pr 42 --dry-run
```

### B. Daemon Service (`openreview serve`)
```bash
# Run local webhook service on port 3000
HOST="127.0.0.1" PORT=3000 \
LLM_PROVIDER="openai" \
SANDBOX_DRIVER="docker" \
openreview serve
```

### C. Root-Cause Investigation & Input Minimizer (`deep-digger` & `deep-digger-min`)
```bash
# Line-level reduction for failing test cases
deep-digger-min --input repro.txt --output minimal.txt --allow-exec -- cargo test --test regression

# Token-level reduction for syntax or JSON payloads
deep-digger-min --input repro.json --output minimal.json --granularity tokens --allow-exec -- cargo test --test regression

# Unified investigation CLI (standalone npm or OpenReview skill)
deep-digger init cases/auth-bug
deep-digger hypothesis cases/auth-bug add H-1 "Buffer overflow on unescaped input"
deep-digger view cases/auth-bug
```

### D. Invariant & Config Auditor (`verify_openreview.py`)
```bash
# Run deterministic invariant audit against repository configuration
python3 skills/openreview/scripts/verify_openreview.py .
```

---

## 5. End-to-End Execution Protocol

When invoked with any PR review or remediation task, execute this sequence:

```text
1. CLASSIFY INTENT
   ├── Review PR -> Enter Mode REVIEW
   ├── Fix Code -> Enter Mode FIX (Verify collaborator permissions)
   ├── Debug Failure -> Enter Mode INVESTIGATE (Hand off to deep-digger)
   └── Configure Bot -> Enter Mode VERIFY_CONFIG

2. INSPECT PR & ENVIRONMENT
   ├── Fetch git diff & changed files
   ├── Detect build system (Cargo, Bun/NPM, Pytest, Go)
   └── Establish baseline test status

3. EXECUTE ENGINE WORKFLOW
   ├── If REVIEW: Run openreview review; format findings via 3-tier taxonomy.
   ├── If FIX: Spin up isolated sandbox, apply minimal patch, execute test verification gate.
   └── If INVESTIGATE: Formulate hypothesis, run deep-digger-min to isolate reproducing lines.

4. GATE & REPORT
   ├── If Tests Pass in FIX: Push attributable commit with verification SHA.
   ├── If Tests Fail in FIX: Block push, report exact runner failure diagnostic.
   └── Deliver clean markdown payload with zero conversational noise.
```

---

## 6. Stop Conditions & Prohibitions

1. **Never bypass verification:** A fix must NEVER be committed or pushed if tests fail, exit non-zero, or produce zero executed test assertions.
2. **Never leak credentials:** RSA keys, GitHub App secrets, and LLM API keys must never be logged, committed, or echoed into PR comments.
3. **Never execute unverified commands outside the sandbox:** Arbitrary shell commands submitted by external PR contributors must only execute within the constrained container sandbox.
4. **Never write speculative documentation:** Ground all findings strictly on the verified local repository files and authoritative Git diffs.
