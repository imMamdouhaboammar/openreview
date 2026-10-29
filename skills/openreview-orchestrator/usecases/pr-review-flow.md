# Use Case: Autonomous PR Review Flow

Step-by-step workflow for executing a read-only code review on a pull request.

---

## 1. Intent Detection
- **Trigger:** `@openreview review` comment on GitHub PR, or local CLI invocation:
  ```bash
  openreview review --repo owner/repo --pr 101 --dry-run
  ```

---

## 2. Execution Procedure

1. **Ingest Authoritative Diff:**
   - Fetch the PR diff using GitHub API or local git tracking branch.
   - Discard binary files, lockfiles, and minified bundles.

2. **Run Read-Only AST & Logic Review:**
   - Mount repository in read-only mode (`SANDBOX_DRIVER=docker` or `local`).
   - Analyze logic correctness, security boundaries, race conditions, and performance bottlenecks.
   - Forbid any file write or command execution.

3. **Map Findings to 3-Tier Taxonomy:**
   - Label critical security vulnerabilities or panics as 🔴 **Blocker**.
   - Label missing validation or performance regressions as 🟡 **Suggestion**.
   - Label documentation or minor clarity improvements as 💭 **Nit**.

4. **Construct GitHub Suggestion Blocks:**
   - Anchor each finding to exact file and line numbers.
   - Formulate drop-in code fixes inside ````suggestion```` markdown blocks.

5. **Publish Review:**
   - Post findings to GitHub PR review thread as a single unified review.
