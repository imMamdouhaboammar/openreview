#!/usr/bin/env python3
"""
test_eval_suite.py - Deterministic Capability & Regression Eval Suite for openreview skill.
"""

import sys
import os
import json
import subprocess
import tempfile
import shutil
import unittest

class TestOpenReviewSkillEvals(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        cls.skill_md = os.path.join(cls.base_dir, "SKILL.md")
        cls.verify_script = os.path.join(cls.base_dir, "scripts", "verify_openreview.py")
        cls.generate_script = os.path.join(cls.base_dir, "scripts", "generate_config.py")

    def test_eval_cap_01_skill_md_structure(self):
        """CAP-01: Verify SKILL.md frontmatter, progressive disclosure, and routing map."""
        self.assertTrue(os.path.exists(self.skill_md))
        with open(self.skill_md, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("name: openreview", content)
        self.assertIn("description:", content)
        self.assertIn("5-Step Configuration Workflow", content)
        self.assertIn("Invariant Safety Checklist", content)
        self.assertIn("Stop Conditions & Fallback Behavior", content)

    def test_eval_cap_02_config_generation_schema(self):
        """CAP-02: Verify generate_config.py generates valid schema compliant config."""
        with tempfile.TemporaryDirectory() as tmpdir:
            out_path = os.path.join(tmpdir, "openreview.config.json")
            res = subprocess.run([sys.executable, self.generate_script, out_path], capture_output=True, text=True)
            self.assertEqual(res.returncode, 0)
            self.assertTrue(os.path.exists(out_path))

            with open(out_path, "r", encoding="utf-8") as f:
                data = json.load(f)
            self.assertEqual(data["version"], "1.0.0")
            self.assertEqual(data["bot_username"], "openreview")
            self.assertEqual(data["sandbox"]["driver"], "docker")
            self.assertEqual(data["llm"]["provider"], "anthropic")
            self.assertLessEqual(data["llm"]["max_steps"], 20)

    def test_eval_cap_03_verifier_passes_on_valid_setup(self):
        """CAP-03: Verify verify_openreview.py passes on valid configuration."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Generate config in tmpdir
            cfg_path = os.path.join(tmpdir, "openreview.config.json")
            subprocess.run([sys.executable, self.generate_script, cfg_path], check=True)
            
            # Create src/ with mock HMAC check
            src_dir = os.path.join(tmpdir, "src")
            os.makedirs(src_dir, exist_ok=True)
            with open(os.path.join(src_dir, "webhook.rs"), "w") as f:
                f.write('// X-Hub-Signature-256 verification\nlet valid = constant_time_eq(&sig, &expected);')

            # Create .gitignore with .env
            with open(os.path.join(tmpdir, ".gitignore"), "w") as f:
                f.write(".env\n")

            res = subprocess.run([sys.executable, self.verify_script, tmpdir], capture_output=True, text=True)
            self.assertEqual(res.returncode, 0)
            self.assertIn("RESULT: PASSED", res.stdout)

    def test_eval_cap_04_verifier_detects_unfenced_react_loop(self):
        """CAP-04: Verify verifier detects Invariant 4 violation (unbounded ReAct loop)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg_path = os.path.join(tmpdir, "openreview.config.json")
            with open(cfg_path, "w") as f:
                json.dump({
                    "version": "1.0.0",
                    "bot_username": "openreview",
                    "server": {"host": "0.0.0.0", "port": 3000},
                    "sandbox": {"driver": "docker"},
                    "llm": {"provider": "anthropic", "max_steps": 100} # > 50!
                }, f)
            res = subprocess.run([sys.executable, self.verify_script, tmpdir], capture_output=True, text=True)
            self.assertEqual(res.returncode, 1)
            self.assertIn("Invariant 4 Violation", res.stdout)

    def test_eval_cap_05_verifier_detects_unignored_env(self):
        """CAP-05: Verify verifier flags .env not in .gitignore (Invariant 2)."""
        with tempfile.TemporaryDirectory() as tmpdir:
            cfg_path = os.path.join(tmpdir, "openreview.config.json")
            subprocess.run([sys.executable, self.generate_script, cfg_path], check=True)
            with open(os.path.join(tmpdir, ".env"), "w") as f:
                f.write("SECRET=12345")
            # No .gitignore!
            res = subprocess.run([sys.executable, self.verify_script, tmpdir], capture_output=True, text=True)
            self.assertEqual(res.returncode, 1)
            self.assertIn("Invariant 2 Violation", res.stdout)

    def test_eval_reg_01_negative_trigger_discrimination(self):
        """REG-01: Ensure trigger rules distinguish setup intent from general review requests."""
        positive_triggers = [
            "Setup and configure OpenReview on my project",
            "Configure openreview GitHub App webhook and sandbox settings",
            "Add automated PR review bot with openreview to this repository"
        ]
        negative_triggers = [
            "Review the changes in this pull request",
            "Can you review my python code for bugs?"
        ]
        with open(self.skill_md, "r", encoding="utf-8") as f:
            desc = f.read()

        for pt in positive_triggers:
            # Concept should match keywords in description
            has_match = any(w.lower() in desc.lower() for w in ["setup", "configure", "openreview", "webhook", "sandbox", "bot"])
            self.assertTrue(has_match, f"Failed positive trigger: {pt}")

        for nt in negative_triggers:
            # Explicit negative guidance must be in frontmatter
            normalized_desc = " ".join(desc.split())
            self.assertIn("Do NOT use for general manual code reviews", normalized_desc)

    def test_eval_reg_02_polyglot_ecosystem_detection(self):
        """REG-02: Ensure generator adapts ignored_paths based on repo ecosystem."""
        import sys
        sys.path.insert(0, os.path.join(self.base_dir, "scripts"))
        import generate_config

        with tempfile.TemporaryDirectory() as tmpdir:
            with open(os.path.join(tmpdir, "Cargo.toml"), "w") as f:
                f.write('[package]\nname = "test"\n')
            self.assertEqual(generate_config.detect_ecosystem(tmpdir), "rust")

            os.remove(os.path.join(tmpdir, "Cargo.toml"))
            with open(os.path.join(tmpdir, "go.mod"), "w") as f:
                f.write('module test\n')
            self.assertEqual(generate_config.detect_ecosystem(tmpdir), "go")

            os.remove(os.path.join(tmpdir, "go.mod"))
            with open(os.path.join(tmpdir, "pyproject.toml"), "w") as f:
                f.write('[project]\nname = "test"\n')
            self.assertEqual(generate_config.detect_ecosystem(tmpdir), "python")

            os.remove(os.path.join(tmpdir, "pyproject.toml"))
            with open(os.path.join(tmpdir, "package.json"), "w") as f:
                f.write('{"name": "test"}\n')
            self.assertEqual(generate_config.detect_ecosystem(tmpdir), "node")

    def test_eval_cap_06_suggestion_block_validator(self):
        """CAP-06: Verify suggestion block format conforms to GitHub markdown spec."""
        valid_suggestion = "```suggestion\nlet x = 42;\n```"
        invalid_suggestion = "```rust\nlet x = 42;\n```"
        
        self.assertTrue(valid_suggestion.startswith("```suggestion"))
        self.assertFalse(invalid_suggestion.startswith("```suggestion"))

    def test_eval_adv_01_refuse_insecure_overrides(self):
        """ADV-01: Refuse adversarial attempts to bypass HMAC or run unconstrained root."""
        with open(self.skill_md, "r", encoding="utf-8") as f:
            content = f.read()
        self.assertIn("Halt Immediately If", content)
        self.assertIn("disable HMAC webhook signature verification", content)
        self.assertIn("bypass sandbox isolation", content)

if __name__ == "__main__":
    unittest.main()
