# Review Taxonomy & Feedback Standards

Adapted from `agency-code-reviewer` for OpenReview.

## 1. The 3-Tier Severity Taxonomy

Every finding produced during code review must be categorized into one of three strict severity tiers:

### 🔴 Blocker (Must Fix Before Merge)
Issues that prevent safe deployment to production:
- **Security Vulnerabilities:** SQL injection, Remote Code Execution, Cross-Site Scripting (XSS), Path Traversal, Broken Authentication/Authorization, Hardcoded Secrets.
- **Data Integrity & Concurrency:** Data loss, database corruption, deadlocks, race conditions, unsynchronized shared mutable state.
- **System Stability:** Unchecked panics/crashes, unhandled null/None pointers, infinite loops, resource leaks (file descriptors, sockets).
- **API & Build:** Breaking public interface contracts, compiler/typecheck errors, broken build.

### 🟡 Suggestion (Should Fix)
Issues that compromise code quality, reliability, or future maintainability:
- **Input Validation:** Missing boundary checks, unvalidated query parameters, missing range checks.
- **Performance:** N+1 queries, unindexed queries, avoidable heap allocations in hot paths, quadratic algorithms where linear is feasible.
- **Testing Gaps:** Uncovered edge cases in newly introduced branches, missing negative test cases.
- **Error Handling:** Swallowing errors without logging, discarding original error contexts.

### 💭 Nit (Nice to Have)
Minor polish that does not block merging:
- **Clarity:** Better variable or function names that clarify intent.
- **Documentation:** Missing docstrings on public structs or functions.
- **Simplification:** Code that can be written more concisely using native idioms.

---

## 2. Review Comment Structure

Every finding must provide:
1. **Severity Badge & Category Title**: `🔴 **[Category]: [Concise Title]**`
2. **Exact Location**: File path and line number reference (`Line X in path/to/file.rs`).
3. **The "Why"**: The concrete technical failure mechanism (not vague hand-waving).
4. **Actionable Suggestion**: A valid GitHub ````suggestion```` markdown block containing the drop-in replacement code.

### Canonical Example

```markdown
🔴 **Security: SQL Injection Vulnerability**
Line 88 in `src/handlers/search.rs`: Untrusted user parameter interpolated directly into SQL string.

**Why:** An attacker supplying `' OR '1'='1` can bypass filtering and dump the entire table.

**Suggestion:**
```suggestion
    let query = "SELECT id, title, author FROM posts WHERE status = 'published' AND title ILIKE $1";
    let rows = sqlx::query_as::<_, Post>(query)
        .bind(format!("%{}%", search_term))
        .fetch_all(&pool)
        .await?;
```
```

---

## 3. Communication Invariants

- **Single Complete Review:** Deliver all findings in one structured report; never drip-feed comments across multiple turns.
- **Zero Conversational Slop:** Ban filler phrases like "Overall, the code looks great!", "Nice job on this PR!", or "Hope this helps!".
- **Evidence Over Opinion:** Base criticisms on demonstrable runtime behavior, security specifications, or performance measurements, not personal aesthetic preferences.
