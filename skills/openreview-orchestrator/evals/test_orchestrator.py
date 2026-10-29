#!/usr/bin/env python3
"""
Test suite for openreview-orchestrator skill.
Tests intent routing, DAG compilation, review format validation, and safety invariants.
"""

from pathlib import Path
import subprocess
import sys
import unittest

BASE_DIR = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = BASE_DIR / "scripts"
sys.path.insert(0, str(SCRIPTS_DIR))

from orchestrate import classify_intent, build_dag  # noqa: E402
from format_review import format_finding, validate_review_markdown  # noqa: E402


class TestOrchestrator(unittest.TestCase):
    def test_classify_intent_review(self):
        self.assertEqual(classify_intent("Please review PR #12 for memory leaks"), "REVIEW")
        self.assertEqual(classify_intent("Audit the latest changes in branch feature"), "REVIEW")
        self.assertEqual(classify_intent("@openreview review"), "REVIEW")

    def test_classify_intent_fix(self):
        self.assertEqual(classify_intent("@openreview fix the missing input validation"), "FIX")
        self.assertEqual(classify_intent("remediate the failing test in users.rs"), "FIX")
        self.assertEqual(classify_intent("patch this buffer overflow"), "FIX")

    def test_classify_intent_investigate(self):
        self.assertEqual(classify_intent("Why did this test fail suddenly?"), "INVESTIGATE")
        self.assertEqual(classify_intent("Minimize this large JSON crash reproducer with ddmin"), "INVESTIGATE")
        self.assertEqual(classify_intent("Isolate the regression in the auth module"), "INVESTIGATE")

    def test_classify_intent_config(self):
        self.assertEqual(classify_intent("Verify openreview config and sandbox setup"), "VERIFY_CONFIG")
        self.assertEqual(classify_intent("Audit invariants in openreview.config.json"), "VERIFY_CONFIG")

    def test_dag_structure_review(self):
        dag = build_dag("REVIEW", "owner/repo", 42)
        self.assertEqual(dag["mode"], "REVIEW")
        self.assertEqual(dag["sandbox"], "read_only")
        self.assertEqual(dag["publish_permission"], "read")
        actions = [s["action"] for s in dag["steps"]]
        self.assertIn("fetch_diff", actions)
        self.assertIn("classify_findings", actions)

    def test_dag_structure_fix(self):
        dag = build_dag("FIX", "owner/repo", 42)
        self.assertEqual(dag["mode"], "FIX")
        self.assertEqual(dag["sandbox"], "writable_ephemeral")
        self.assertEqual(dag["publish_permission"], "write")
        actions = [s["action"] for s in dag["steps"]]
        self.assertIn("check_authorization", actions)
        self.assertIn("verification_gate", actions)

    def test_format_finding_canonical(self):
        finding = format_finding(
            severity="BLOCKER",
            category="Security",
            title="SQL Injection",
            file_path="src/db.rs",
            line=42,
            why="Unescaped query parameters can execute arbitrary SQL.",
            suggestion="let q = sqlx::query!(\"SELECT * FROM users WHERE id = $1\", id);"
        )
        self.assertIn("🔴 **Security: SQL Injection**", finding)
        self.assertIn("Line 42 in `src/db.rs`", finding)
        self.assertIn("```suggestion", finding)
        
        errors = validate_review_markdown(finding)
        self.assertEqual(errors, [])

    def test_validate_review_detects_missing_badge(self):
        bad_review = "This looks like a bug on line 12."
        errors = validate_review_markdown(bad_review)
        self.assertTrue(any("Missing required severity badge" in e for e in errors))

    def test_skill_manifest_exists(self):
        skill_md = BASE_DIR / "SKILL.md"
        self.assertTrue(skill_md.exists())
        content = skill_md.read_text(encoding="utf-8")
        self.assertIn("name: openreview-orchestrator", content)
        self.assertIn("Operating Modes & System DAG", content)
        self.assertIn("Review Taxonomy & Feedback Standards", content)


if __name__ == "__main__":
    unittest.main()
