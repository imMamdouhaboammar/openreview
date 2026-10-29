"""Deterministic tool tests, independent of unrun agent behavior evaluations."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from deep_digger import main, ddmin, classify_case, run_command  # noqa: E402


def invoke(*args):
    output = io.StringIO()
    with contextlib.redirect_stdout(output):
        rc = main(list(args))
    return rc, output.getvalue()


class CliTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.case = self.root / "case"
        self.assertEqual(0, invoke("init", str(self.case), "--title", "Broken total", "--symptom", "wrong output")[0])

    def test_hypothesis_and_evidence_keep_integrity(self):
        rc, out = invoke("hypothesis", str(self.case), "--claim", "cache key collision", "--test", "vary key")
        self.assertEqual(0, rc)
        hypothesis_id = json.loads(out)["id"]
        source = self.root / "trace.log"
        source.write_text("trace sanitized", encoding="utf-8")
        rc, out = invoke("evidence", str(self.case), "--hypothesis", hypothesis_id, "--effect", "supports", "--kind", "observation", "--note", "same key", "--source", str(source))
        self.assertEqual(0, rc)
        record = json.loads(out)
        self.assertEqual(64, len(record["sha256"]))
        self.assertEqual("unverified", classify_case(json.loads((self.case / "case.json").read_text()), hypothesis_id)["status"])
        self.assertNotEqual(0, invoke("evidence", str(self.case), "--hypothesis", "H-unknown", "--effect", "supports", "--kind", "observation", "--note", "wrong")[0])

    def test_missing_causal_gates_not_confirmed(self):
        _, out = invoke("hypothesis", str(self.case), "--claim", "bad parser", "--test", "replace parser")
        hid = json.loads(out)["id"]
        rc, result = invoke("report", str(self.case), "--json")
        self.assertEqual(0, rc)
        obj = json.loads(result)
        self.assertEqual("unverified", obj["hypotheses"][0]["status"])
        self.assertTrue(obj["hypotheses"][0]["missing_gates"])

    def test_experiment_requires_opt_in(self):
        _, out = invoke("hypothesis", str(self.case), "--claim", "bad parser", "--test", "replace parser")
        hid = json.loads(out)["id"]
        rc, _ = invoke("experiment", str(self.case), "--hypothesis", hid, "--phase", "baseline", "--", sys.executable, "-c", "raise SystemExit(1)")
        self.assertNotEqual(0, rc)

    def test_reproduction_then_intervention_reversal_and_control(self):
        _, out = invoke("hypothesis", str(self.case), "--claim", "incompatible flag", "--test", "toggle flag")
        hid = json.loads(out)["id"]
        for phase, expected, runs in (("baseline", 1, 2), ("intervention", 0, 2), ("reversal", 1, 1), ("negative-control", 1, 1)):
            rc, _ = invoke("experiment", str(self.case), "--hypothesis", hid, "--phase", phase, "--repeats", str(runs), "--allow-exec", "--", sys.executable, "-c", f"raise SystemExit({expected})")
            self.assertEqual(0, rc)
        report = json.loads(invoke("report", str(self.case), "--json")[1])
        item = report["hypotheses"][0]
        self.assertEqual("experimental-pattern", item["status"])
        self.assertEqual([], item["missing_gates"])
        next_actions = json.loads(invoke("next", str(self.case))[1])["next_actions"]
        self.assertIn("manually", next_actions[0]["next"])

    def test_flaky_baseline_is_not_confirmed(self):
        _, out = invoke("hypothesis", str(self.case), "--claim", "scheduler race", "--test", "control scheduling")
        hid = json.loads(out)["id"]
        # A mixed run is recorded as conflicting evidence, not as stable reproduction.
        data = json.loads((self.case / "case.json").read_text())
        data["experiments"] = [{"hypothesis": hid, "phase": "baseline", "outcomes": ["FAIL", "PASS"]}]
        result = classify_case(data, hid)
        self.assertEqual("inconclusive", result["status"])

    def test_minimizer_is_one_minimal_and_preserves_interactions(self):
        chunks = ["noise1\n", "BUG\n", "noise2\n", "TRIGGER\n", "noise3\n"]
        tested = []
        def oracle(lines):
            tested.append(lines)
            return "FAIL" if "BUG\n" in lines and "TRIGGER\n" in lines else "PASS"
        output, complete = ddmin(chunks, oracle, 100)
        self.assertTrue(complete)
        self.assertEqual(["BUG\n", "TRIGGER\n"], output)
        self.assertGreater(len(tested), 0)

    def test_minimizer_budget_does_not_pretend_complete(self):
        chunks = ["x\n", "BUG\n", "y\n", "TRIGGER\n"]
        output, complete = ddmin(chunks, lambda c: "FAIL" if "BUG\n" in c and "TRIGGER\n" in c else "PASS", 1)
        self.assertFalse(complete)
        self.assertLessEqual(len(output), len(chunks))

    def test_single_line_reduces_to_empty_when_empty_still_fails(self):
        output, complete = ddmin(["irrelevant\n"], lambda _: "FAIL", 20)
        self.assertEqual([], output)
        self.assertTrue(complete)

    def test_nonfinite_timeout_is_rejected(self):
        for value in (float("inf"), float("nan"), 601.0, 0.0):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    run_command([sys.executable, "-c", "pass"], value)

    def test_init_refuses_overwrite(self):
        self.assertNotEqual(0, invoke("init", str(self.case), "--title", "new", "--symptom", "new")[0])
        self.assertIn("Broken total", (self.case / "case.json").read_text())

    def test_evidence_hash_drift_detected(self):
        _, out = invoke("hypothesis", str(self.case), "--claim", "stale input", "--test", "compare source")
        hid = json.loads(out)["id"]
        source = self.root / "sanitized.log"
        source.write_text("original", encoding="utf-8")
        self.assertEqual(0, invoke("evidence", str(self.case), "--hypothesis", hid, "--effect", "supports", "--kind", "observation", "--note", "capture", "--source", str(source))[0])
        self.assertEqual(0, invoke("audit", str(self.case))[0])
        source.write_text("mutated", encoding="utf-8")
        self.assertEqual(2, invoke("audit", str(self.case))[0])

    def test_minimize_command_integration(self):
        source = self.root / "failing.txt"
        source.write_text("noise\nBUG\nnoisy\nTRIGGER\nend\n", encoding="utf-8")
        dest = self.root / "minimal.txt"
        oracle = Path(__file__).parent / "fixtures" / "trigger_oracle.py"
        rc, out = invoke("minimize", "--input", str(source), "--output", str(dest),
                         "--repeats", "2", "--allow-exec", "--", sys.executable, str(oracle), "{input}")
        self.assertEqual(0, rc)
        self.assertTrue(json.loads(out)["one_minimal_verified"])
        self.assertEqual("BUG\nTRIGGER\n", dest.read_text(encoding="utf-8"))
        self.assertIn("noise", source.read_text(encoding="utf-8"))

    def test_minimize_recognizes_final_line_without_newline(self):
        source = self.root / "without_newline.txt"
        source.write_text("noise\nBUG\nTRIGGER", encoding="utf-8")
        dest = self.root / "without_newline_out.txt"
        oracle = Path(__file__).parent / "fixtures" / "trigger_oracle.py"
        rc, out = invoke("minimize", "--input", str(source), "--output", str(dest),
                         "--repeats", "2", "--allow-exec", "--", sys.executable, str(oracle), "{input}")
        self.assertEqual(0, rc)
        self.assertTrue(json.loads(out)["one_minimal_verified"])
        self.assertEqual("BUG\nTRIGGER", dest.read_text(encoding="utf-8"))

    def test_missing_output_parent_rejected_before_oracle_execution(self):
        source = self.root / "failing.txt"
        source.write_text("BUG\nTRIGGER\n", encoding="utf-8")
        dest = self.root / "missing" / "result.txt"
        oracle = Path(__file__).parent / "fixtures" / "trigger_oracle.py"
        with mock.patch("deep_digger.run_command") as runner:
            rc, _ = invoke("minimize", "--input", str(source), "--output", str(dest),
                           "--repeats", "2", "--allow-exec", "--", sys.executable, str(oracle), "{input}")
        self.assertEqual(2, rc)
        runner.assert_not_called()

    def test_non_matching_oracle_refuses_minimize(self):
        source = self.root / "passing.txt"
        source.write_text("fine\n", encoding="utf-8")
        dest = self.root / "would_be_written.txt"
        oracle = Path(__file__).parent / "fixtures" / "trigger_oracle.py"
        self.assertEqual(2, invoke("minimize", "--input", str(source), "--output", str(dest),
                                    "--repeats", "2", "--allow-exec", "--", sys.executable, str(oracle), "{input}")[0])
        self.assertFalse(dest.exists())


if __name__ == "__main__":
    unittest.main()
