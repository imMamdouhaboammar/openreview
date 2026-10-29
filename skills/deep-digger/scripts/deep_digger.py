#!/usr/bin/env python3
"""Deep Digger: evidence ledger, controlled test receipts, and delta debugging.

This tool records observations and experiments; it does not infer causality on
its own. Python standard library only. No shell execution or network access.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Callable, Sequence
from uuid import uuid4


PHASES = ("baseline", "intervention", "reversal", "negative-control")
VERDICTS = {0: "PASS", 1: "FAIL"}
GATES = {
    "baseline": ("FAIL", 2),
    "intervention": ("PASS", 2),
    "reversal": ("FAIL", 1),
    "negative-control": ("FAIL", 1),
}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def fingerprint(text: bytes) -> str:
    return hashlib.sha256(text).hexdigest()


def read_case(folder: str) -> dict:
    file = Path(folder) / "case.json"
    data = json.loads(file.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise ValueError("unsupported case schema version")
    return data


def save_case(folder: str, data: dict) -> None:
    path = Path(folder) / "case.json"
    # Atomic replacement avoids half-written ledgers after interruption.
    descriptor, temporary = tempfile.mkstemp(prefix=".deep-digger-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(data, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def require_hypothesis(data: dict, hypothesis: str) -> None:
    if hypothesis not in {item["id"] for item in data["hypotheses"]}:
        raise ValueError(f"unknown hypothesis: {hypothesis}")


def classify_case(data: dict, hypothesis: str) -> dict:
    """Evidence gate, not numerical certainty or an automated causal oracle."""
    relevant = [x for x in data.get("experiments", []) if x["hypothesis"] == hypothesis]
    evidence = [x for x in data.get("evidence", []) if x["hypothesis"] == hypothesis]
    missing = []
    conflicting = []
    for phase, (expected, required) in GATES.items():
        outcomes = [v for x in relevant if x["phase"] == phase for v in x["outcomes"]]
        if len(outcomes) < required:
            missing.append(f"{phase}: require {required} consistent {expected} runs")
        if outcomes and any(outcome != expected for outcome in outcomes):
            conflicting.append(f"{phase}: unexpected verdicts {outcomes}")
    if any(e["effect"] == "refutes" for e in evidence):
        conflicting.append("contradictory evidence not reconciled")
    baseline = [v for x in relevant if x["phase"] == "baseline" for v in x["outcomes"]]
    if conflicting:
        status = "inconclusive"
    elif not missing:
        status = "experimental-pattern"
    elif len(baseline) >= 2 and all(outcome == "FAIL" for outcome in baseline):
        status = "reproduced"
    else:
        status = "unverified"
    return {"status": status, "missing_gates": missing, "conflicts": conflicting,
            "supporting_evidence": [e["id"] for e in evidence if e["effect"] == "supports"],
            "refuting_evidence": [e["id"] for e in evidence if e["effect"] == "refutes"],
            "causal_scope": "controlled experiments only; mechanism and outside-scope causes may remain unknown"}


def ddmin(chunks: Sequence[str], oracle: Callable[[list[str]], str], max_tests: int = 200) -> tuple[list[str], bool]:
    """Minimize a stable failing sequence to 1-minimal under the given oracle.

    Returns (best known failing input, completed). UNRESOLVED is never failure.
    An upper-bound test budget prevents unlimited loops.
    """
    if max_tests < 1:
        raise ValueError("max_tests must be >= 1")
    current = list(chunks)
    count = 0

    def check(candidate: list[str]) -> str:
        nonlocal count
        if count >= max_tests:
            return "BUDGET"
        count += 1
        verdict = oracle(candidate)
        if verdict not in {"PASS", "FAIL", "UNRESOLVED"}:
            raise ValueError(f"invalid oracle verdict: {verdict}")
        return verdict

    baseline = check(current)
    if baseline != "FAIL":
        raise ValueError("baseline must deterministically FAIL before minimization")
    if not current:
        return current, True
    granularity = 2
    while len(current) >= 2:
        unit_size = math.ceil(len(current) / granularity)
        reduced = False
        for start in range(0, len(current), unit_size):
            complement = current[:start] + current[start + unit_size:]
            verdict = check(complement)
            if verdict == "BUDGET":
                return current, False
            if verdict == "FAIL":
                current = complement
                granularity = max(2, granularity - 1)
                reduced = True
                break
        if reduced:
            continue
        if granularity >= len(current):
            break
        granularity = min(len(current), granularity * 2)
    # Explicit 1-minimality check, even when an unresolved complement appeared.
    for index in range(len(current)):
        verdict = check(current[:index] + current[index + 1:])
        if verdict in {"BUDGET", "UNRESOLVED"}:
            return current, False
        if verdict == "FAIL":
            # A changing oracle or external state invalidates closure.
            return current, False
    return current, True


def normalize_command(argv: list[str]) -> list[str]:
    args = argv[1:] if argv and argv[0] == "--" else argv
    if not args or not args[0].strip():
        raise ValueError("command required after --")
    # Never interpret shell syntax or compose a command string.
    return args


def run_command(argv: list[str], timeout: float) -> dict:
    if timeout <= 0:
        raise ValueError("timeout must be positive")
    try:
        result = subprocess.run(argv, shell=False, timeout=timeout,
                                capture_output=True, check=False)
        # Capture hashes, never raw output (which may contain secrets).
        return {"verdict": VERDICTS.get(result.returncode, "UNRESOLVED"),
                "exit_code": result.returncode,
                "stdout_sha256": fingerprint(result.stdout),
                "stderr_sha256": fingerprint(result.stderr)}
    except (OSError, subprocess.TimeoutExpired) as err:
        return {"verdict": "UNRESOLVED", "error_type": type(err).__name__}


def cmd_init(args) -> None:
    folder = Path(args.folder)
    if folder.exists():
        raise ValueError("case directory already exists; refusing overwrite")
    folder.mkdir(parents=True, exist_ok=False)
    data = {"schema_version": 1, "id": str(uuid4()), "created_at": utc_now(),
            "title": args.title, "symptom": args.symptom,
            "hypotheses": [], "evidence": [], "experiments": []}
    save_case(str(folder), data)
    print(json.dumps({"case": str(folder), "id": data["id"]}))


def cmd_hypothesis(args) -> None:
    data = read_case(args.folder)
    record = {"id": f"H-{len(data['hypotheses']) + 1}", "claim": args.claim,
              "discriminating_test": args.test, "created_at": utc_now()}
    data["hypotheses"].append(record)
    save_case(args.folder, data)
    print(json.dumps(record, ensure_ascii=False))


def cmd_evidence(args) -> None:
    data = read_case(args.folder)
    require_hypothesis(data, args.hypothesis)
    source = Path(args.source).resolve() if args.source else None
    if source and (not source.is_file() or not source.exists()):
        raise ValueError("source must be an existing readable file")
    record = {"id": f"E-{len(data['evidence']) + 1}", "hypothesis": args.hypothesis,
              "kind": args.kind, "effect": args.effect, "note": args.note,
              "source": str(source) if source else None,
              "sha256": sha256(source) if source else None, "created_at": utc_now()}
    data["evidence"].append(record)
    save_case(args.folder, data)
    print(json.dumps(record, ensure_ascii=False))


def cmd_experiment(args) -> None:
    if not args.allow_exec:
        raise ValueError("execution requires --allow-exec and an isolated, user-approved command")
    if not 1 <= args.repeats <= 30:
        raise ValueError("repeats must be between 1 and 30")
    data = read_case(args.folder)
    require_hypothesis(data, args.hypothesis)
    argv = normalize_command(args.command)
    runs = [run_command(argv, args.timeout) for _ in range(args.repeats)]
    # Do not store CLI arguments; they might contain secrets. A command digest
    # permits correlation without disclosing the argument values.
    command_digest = fingerprint(json.dumps(argv, ensure_ascii=False).encode())
    record = {"id": f"X-{len(data['experiments']) + 1}", "hypothesis": args.hypothesis,
              "phase": args.phase, "time": utc_now(), "command_sha256": command_digest,
              "outcomes": [run["verdict"] for run in runs], "runs": runs}
    data["experiments"].append(record)
    save_case(args.folder, data)
    print(json.dumps({"experiment": record["id"], "phase": record["phase"],
                      "outcomes": record["outcomes"]}))


def cmd_report(args) -> None:
    data = read_case(args.folder)
    result = {"title": data["title"], "symptom": data["symptom"], "case_id": data["id"],
              "hypotheses": [{**h, **classify_case(data, h["id"])} for h in data["hypotheses"]],
              "experiment_count": len(data["experiments"]),
              "evidence_count": len(data["evidence"]),
              "evidence": data["evidence"], "experiments": data["experiments"]}
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return
    print(f"# Deep Digger: {result['title']}\n\nSymptom: {result['symptom']}\n")
    for h in result["hypotheses"]:
        print(f"## {h['id']} {h['claim']}\n\nStatus: {h['status']}\n")
        print("Discriminating test: " + h["discriminating_test"])
        for reason in h["missing_gates"] + h["conflicts"]:
            print("- " + reason)
        print("Evidence: " + (", ".join(h["supporting_evidence"] + h["refuting_evidence"]) or "none"))
        print("\nConclusion is limited to the documented experimental scope.\n")
    if data["experiments"]:
        print("## Experiment receipts\n")
        for experiment in data["experiments"]:
            print(f"- {experiment['id']} / {experiment['hypothesis']} / {experiment['phase']}: "
                  + ", ".join(experiment["outcomes"]))
    if data["evidence"]:
        print("\n## Source evidence\n")
        for evidence in data["evidence"]:
            print(f"- {evidence['id']} ({evidence['effect']}): {evidence['note']}; "
                  f"source_sha256={evidence['sha256'] or 'not supplied'}")


def cmd_next(args) -> None:
    """Prioritize missing/disputed verification; do not invent hypotheses."""
    data = read_case(args.folder)
    tasks = []
    for hypothesis in data["hypotheses"]:
        state = classify_case(data, hypothesis["id"])
        if state["conflicts"]:
            action = "Investigate contradictions with an independent oracle before any causal claim"
        elif state["missing_gates"]:
            action = f"Plan a safe discriminating experiment: {state['missing_gates'][0]}"
        else:
            action = "Review mechanism, confounders and alternative hypotheses manually"
        tasks.append({"hypothesis": hypothesis["id"], "claim": hypothesis["claim"],
                      "status": state["status"], "next": action,
                      "discriminator": hypothesis["discriminating_test"]})
    print(json.dumps({"next_actions": tasks}, ensure_ascii=False, indent=2))


def cmd_audit(args) -> None:
    data = read_case(args.folder)
    drift = []
    for ev in data["evidence"]:
        if not ev.get("source"):
            continue
        try:
            if sha256(Path(ev["source"])) != ev["sha256"]:
                drift.append({"id": ev["id"], "reason": "content changed"})
        except OSError:
            drift.append({"id": ev["id"], "reason": "source missing or unreadable"})
    print(json.dumps({"evidence_checked": len(data["evidence"]), "drift": drift}))
    if drift:
        raise ValueError("evidence drift detected")


def cmd_minimize(args) -> None:
    if not args.allow_exec:
        raise ValueError("minimization runs commands; explicitly pass --allow-exec")
    if not 2 <= args.repeats <= 10:
        raise ValueError("repeats must be between 2 and 10 for a stability check")
    if args.max_tests <= 0:
        raise ValueError("max-tests must be positive")
    input_path, output_path = Path(args.input), Path(args.output)
    if input_path.resolve() == output_path.resolve():
        raise ValueError("output must not overwrite input")
    if not input_path.is_file() or output_path.exists():
        raise ValueError("input must be a file and output must not exist")
    argv = normalize_command(args.command)
    if not any("{input}" in item for item in argv):
        raise ValueError("command must include {input} placeholder")
    # newline='' preserves line terminators on all platforms.
    with input_path.open("r", encoding="utf-8", newline="") as stream:
        lines = stream.readlines()
    with tempfile.TemporaryDirectory(prefix="deep-digger-min-") as temporary:
        candidate_path = Path(temporary) / "candidate.txt"

        def oracle(candidate: list[str]) -> str:
            with candidate_path.open("w", encoding="utf-8", newline="") as stream:
                stream.writelines(candidate)
            invocation = [item.replace("{input}", str(candidate_path)) for item in argv]
            outcomes = [run_command(invocation, args.timeout)["verdict"] for _ in range(args.repeats)]
            return outcomes[0] if len(set(outcomes)) == 1 else "UNRESOLVED"

        minimized, complete = ddmin(lines, oracle, args.max_tests)
    with output_path.open("x", encoding="utf-8", newline="") as stream:
        stream.writelines(minimized)
    print(json.dumps({"output": str(output_path), "original_lines": len(lines),
                      "retained_lines": len(minimized), "one_minimal_verified": complete,
                      "meaning": "1-minimal only for the tested oracle, not global root cause"}))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Deep Digger evidence and causal experiment CLI")
    sub = parser.add_subparsers(dest="action", required=True)
    init = sub.add_parser("init", help="Create a new case ledger")
    init.add_argument("folder"); init.add_argument("--title", required=True); init.add_argument("--symptom", required=True)
    init.set_defaults(func=cmd_init)
    hyp = sub.add_parser("hypothesis", help="Record a falsifiable root-cause hypothesis")
    hyp.add_argument("folder"); hyp.add_argument("--claim", required=True); hyp.add_argument("--test", required=True)
    hyp.set_defaults(func=cmd_hypothesis)
    ev = sub.add_parser("evidence", help="Record evidence and optional source hash")
    ev.add_argument("folder"); ev.add_argument("--hypothesis", required=True)
    ev.add_argument("--kind", choices=("observation", "mechanism", "counterexample", "reference"), required=True)
    ev.add_argument("--effect", choices=("supports", "refutes", "neutral"), required=True)
    ev.add_argument("--note", required=True); ev.add_argument("--source")
    ev.set_defaults(func=cmd_evidence)
    exp = sub.add_parser("experiment", help="Record repeated command outcomes; explicit execution opt-in")
    exp.add_argument("folder"); exp.add_argument("--hypothesis", required=True)
    exp.add_argument("--phase", choices=PHASES, required=True)
    exp.add_argument("--repeats", type=int, default=2); exp.add_argument("--timeout", type=float, default=15)
    exp.add_argument("--allow-exec", action="store_true")
    exp.set_defaults(func=cmd_experiment)
    rep = sub.add_parser("report", help="Print evidence and causal gates")
    rep.add_argument("folder"); rep.add_argument("--json", action="store_true")
    rep.set_defaults(func=cmd_report)
    nxt = sub.add_parser("next", help="Return next discriminating or verification step")
    nxt.add_argument("folder"); nxt.set_defaults(func=cmd_next)
    audit = sub.add_parser("audit", help="Check current source hashes against captured evidence")
    audit.add_argument("folder"); audit.set_defaults(func=cmd_audit)
    mini = sub.add_parser("minimize", help="Reduce a stable failing text input using an oracle")
    mini.add_argument("--input", required=True); mini.add_argument("--output", required=True)
    mini.add_argument("--repeats", type=int, default=3); mini.add_argument("--max-tests", type=int, default=200)
    mini.add_argument("--timeout", type=float, default=15)
    mini.add_argument("--allow-exec", action="store_true")
    mini.set_defaults(func=cmd_minimize)
    return parser


def main(argv: list[str] | None = None) -> int:
    try:
        original = list(sys.argv[1:] if argv is None else argv)
        if original and original[0] in {"experiment", "minimize"}:
            if "--" not in original:
                raise ValueError("a command must follow the -- delimiter")
            cut = original.index("--")
            original, command = original[:cut], original[cut + 1:]
        else:
            command = []
        args = build_parser().parse_args(original)
        args.command = command
        args.func(args)
        return 0
    except SystemExit as exc:
        return int(exc.code)
    except (ValueError, OSError, json.JSONDecodeError, UnicodeError) as exc:
        print(f"Deep Digger: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
