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
import re
import subprocess
import sys
import tempfile
from typing import Callable, Sequence
from uuid import uuid4


PHASES = ("baseline", "intervention", "reversal", "negative-control")
VERDICTS = {0: "PASS", 1: "FAIL"}
MAX_ORACLE_TIMEOUT = 600.0
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
    if "causal_links" not in data:
        data["causal_links"] = []
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
    while current:
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
    validate_timeout(timeout)
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


def validate_timeout(timeout: float) -> None:
    if not math.isfinite(timeout) or not 0 < timeout <= MAX_ORACLE_TIMEOUT:
        raise ValueError(f"timeout must be finite and between 0 and {MAX_ORACLE_TIMEOUT:g} seconds")


def cmd_init(args) -> None:
    folder = Path(args.folder)
    if folder.exists():
        raise ValueError("case directory already exists; refusing overwrite")
    folder.mkdir(parents=True, exist_ok=False)
    data = {"schema_version": 1, "id": str(uuid4()), "created_at": utc_now(),
            "title": args.title, "symptom": args.symptom,
            "hypotheses": [], "evidence": [], "experiments": [], "causal_links": []}
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
    validate_timeout(args.timeout)
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


def analyze_trace_payload(data: any) -> dict:
    spans = []
    if isinstance(data, list):
        spans = data
    elif isinstance(data, dict):
        if "spans" in data and isinstance(data["spans"], list):
            spans = data["spans"]
        elif "log" in data and isinstance(data["log"], dict) and "entries" in data["log"]:
            entries = data["log"]["entries"]
            failing = []
            for entry in entries:
                if isinstance(entry, dict):
                    status = entry.get("response", {}).get("status", 0)
                    if status >= 400:
                        url = entry.get("request", {}).get("url", "unknown")
                        failing.append({"name": url, "status": f"HTTP {status}", "time_ms": entry.get("time", 0)})
            return {
                "format": "HAR",
                "total_items": len(entries),
                "failing_count": len(failing),
                "failing_items": failing,
                "summary": f"{len(failing)} failing HTTP requests across {len(entries)} entries"
            }
        else:
            spans = [data]

    failing = []
    for s in spans:
        if not isinstance(s, dict):
            continue
        name = s.get("name") or s.get("operation_name") or s.get("title") or "span"
        status = s.get("status", {})
        status_code = ""
        error_desc = ""
        if isinstance(status, dict):
            status_code = str(status.get("status_code") or status.get("code") or "")
            error_desc = str(status.get("description") or status.get("message") or "")
        elif isinstance(status, (str, int)):
            status_code = str(status)

        events = s.get("events", [])
        exceptions = []
        if isinstance(events, list):
            for ev in events:
                if isinstance(ev, dict) and ("exception" in str(ev.get("name", "")).lower() or "error" in str(ev.get("name", "")).lower()):
                    attrs = ev.get("attributes", {})
                    exc_type = attrs.get("exception.type") or attrs.get("type") or "Exception"
                    exc_msg = attrs.get("exception.message") or attrs.get("message") or ""
                    exceptions.append(f"{exc_type}: {exc_msg}".strip(": "))

        is_error = (
            status_code.upper() in {"ERROR", "2", "FAIL", "FAILED"} or
            s.get("error") is True or
            bool(exceptions)
        )
        if is_error:
            failing.append({
                "name": name,
                "status": status_code or "ERROR",
                "description": error_desc or (", ".join(exceptions) if exceptions else "Error in span")
            })

    return {
        "format": "OpenTelemetry/SpanTrace",
        "total_items": len(spans),
        "failing_count": len(failing),
        "failing_items": failing,
        "summary": f"{len(failing)} error span(s) detected across {len(spans)} span(s)"
    }


def check_causal_cycle(links: list[dict], new_parent: str, new_child: str) -> None:
    adj: dict[str, list[str]] = {}
    for link in links:
        adj.setdefault(link["parent"], []).append(link["child"])
    adj.setdefault(new_parent, []).append(new_child)

    visited = set()
    rec_stack = set()

    def dfs(node: str) -> bool:
        visited.add(node)
        rec_stack.add(node)
        for neighbor in adj.get(node, []):
            if neighbor not in visited:
                if dfs(neighbor):
                    return True
            elif neighbor in rec_stack:
                return True
        rec_stack.remove(node)
        return False

    for node in list(adj.keys()):
        if node not in visited:
            if dfs(node):
                raise ValueError(f"adding causal link {new_parent} -> {new_child} introduces a cycle")


def cmd_trace(args) -> None:
    data = read_case(args.folder)
    require_hypothesis(data, args.hypothesis)
    trace_path = Path(args.file).resolve()
    if not trace_path.is_file():
        raise ValueError(f"trace file not found: {trace_path}")
    raw_content = trace_path.read_text(encoding="utf-8")
    digest = sha256(trace_path)
    try:
        trace_data = json.loads(raw_content)
    except json.JSONDecodeError as err:
        raise ValueError(f"trace file is not valid JSON: {err}")
    findings = analyze_trace_payload(trace_data)
    record = {
        "id": f"E-{len(data['evidence']) + 1}",
        "hypothesis": args.hypothesis,
        "kind": "observation",
        "effect": "supports" if findings["failing_count"] > 0 else "neutral",
        "note": f"Trace analysis: {findings['summary']}",
        "source": str(trace_path),
        "sha256": digest,
        "created_at": utc_now(),
        "trace_metadata": findings
    }
    data["evidence"].append(record)
    save_case(args.folder, data)
    print(json.dumps(record, ensure_ascii=False, indent=2))


def cmd_link(args) -> None:
    data = read_case(args.folder)
    require_hypothesis(data, args.parent)
    require_hypothesis(data, args.child)
    if args.parent == args.child:
        raise ValueError("cannot link hypothesis to itself")
    if "causal_links" not in data:
        data["causal_links"] = []
    check_causal_cycle(data["causal_links"], args.parent, args.child)
    link = {
        "parent": args.parent,
        "child": args.child,
        "relation": args.relation,
        "created_at": utc_now()
    }
    data["causal_links"].append(link)
    save_case(args.folder, data)
    print(json.dumps(link, ensure_ascii=False, indent=2))


def cmd_flake(args) -> None:
    if not args.allow_exec:
        raise ValueError("flake evaluation runs commands; explicitly pass --allow-exec")
    if not 3 <= args.repeats <= 100:
        raise ValueError("repeats must be between 3 and 100")
    validate_timeout(args.timeout)
    data = read_case(args.folder)
    require_hypothesis(data, args.hypothesis)
    argv = normalize_command(args.command)
    runs = [run_command(argv, args.timeout) for _ in range(args.repeats)]
    fails = sum(1 for r in runs if r["verdict"] == "FAIL")
    passes = sum(1 for r in runs if r["verdict"] == "PASS")
    unresolved = sum(1 for r in runs if r["verdict"] == "UNRESOLVED")
    failure_rate = fails / args.repeats

    if failure_rate == 1.0:
        classification = "DETERMINISTIC_FAILURE"
    elif failure_rate == 0.0:
        classification = "ZERO_FAILURE"
    elif unresolved > 0:
        classification = "UNSTABLE_ENVIRONMENT"
    else:
        classification = "INTERMITTENT_FLAKE"

    result = {
        "hypothesis": args.hypothesis,
        "repeats": args.repeats,
        "fails": fails,
        "passes": passes,
        "unresolved": unresolved,
        "empirical_failure_rate": round(failure_rate, 4),
        "classification": classification,
        "command_sha256": fingerprint(json.dumps(argv, ensure_ascii=False).encode()),
        "time": utc_now()
    }

    record = {
        "id": f"E-{len(data['evidence']) + 1}",
        "hypothesis": args.hypothesis,
        "kind": "observation",
        "effect": "supports" if failure_rate > 0 else "neutral",
        "note": f"Flake analysis: {classification} (failure_rate={failure_rate:.1%}, runs={args.repeats})",
        "source": None,
        "sha256": None,
        "created_at": utc_now(),
        "flake_metrics": result
    }
    data["evidence"].append(record)
    save_case(args.folder, data)
    print(json.dumps(result, ensure_ascii=False, indent=2))


def generate_html_report(result: dict) -> str:
    title = result.get("title", "Investigation Case")
    symptom = result.get("symptom", "N/A")
    case_id = result.get("case_id", "N/A")
    hypotheses = result.get("hypotheses", [])
    causal_links = result.get("causal_links", [])
    experiments = result.get("experiments", [])
    evidence = result.get("evidence", [])

    status_colors = {
        "experimental-pattern": "#10b981",
        "reproduced": "#3b82f6",
        "unverified": "#f59e0b",
        "inconclusive": "#ef4444"
    }

    cards_html = []
    for h in hypotheses:
        color = status_colors.get(h["status"], "#6b7280")
        missing_items = "".join(f"<li>{r}</li>" for r in (h["missing_gates"] + h["conflicts"])) or "<li>None (All criteria met)</li>"
        ev_items = ", ".join(h["supporting_evidence"] + h["refuting_evidence"]) or "None"
        cards_html.append(f"""
        <div class="card">
            <div class="card-header">
                <h3>{h["id"]}: {h["claim"]}</h3>
                <span class="badge" style="background-color: {color};">{h["status"].upper()}</span>
            </div>
            <p><strong>Discriminating Test:</strong> <code>{h["discriminating_test"]}</code></p>
            <p><strong>Gate Status:</strong></p>
            <ul>{missing_items}</ul>
            <p><strong>Attached Evidence:</strong> {ev_items}</p>
        </div>
        """)

    exp_rows = []
    for exp in experiments:
        outcomes_badges = " ".join(f'<span class="badge" style="background: {"#10b981" if o=="PASS" else "#ef4444" if o=="FAIL" else "#f59e0b"};">{o}</span>' for o in exp["outcomes"])
        exp_rows.append(f"""
        <tr>
            <td><code>{exp["id"]}</code></td>
            <td><code>{exp["hypothesis"]}</code></td>
            <td><strong>{exp["phase"]}</strong></td>
            <td>{outcomes_badges}</td>
            <td><code>{exp["time"]}</code></td>
        </tr>
        """)

    ev_rows = []
    for ev in evidence:
        effect_color = "#10b981" if ev["effect"] == "supports" else "#ef4444" if ev["effect"] == "refutes" else "#6b7280"
        ev_rows.append(f"""
        <tr>
            <td><code>{ev["id"]}</code></td>
            <td><code>{ev["hypothesis"]}</code></td>
            <td><span class="badge" style="background: {effect_color};">{ev["effect"]}</span></td>
            <td>{ev["kind"]}</td>
            <td>{ev["note"]}</td>
            <td><code>{ev["sha256"][:12] if ev["sha256"] else "N/A"}...</code></td>
        </tr>
        """)

    svg_elements = []
    if causal_links or hypotheses:
        node_coords = {}
        for idx, h in enumerate(hypotheses):
            cx = 100 + (idx % 3) * 220
            cy = 80 + (idx // 3) * 120
            node_coords[h["id"]] = (cx, cy)
            color = status_colors.get(h["status"], "#6b7280")
            svg_elements.append(f"""
            <g class="dag-node">
                <circle cx="{cx}" cy="{cy}" r="32" fill="#1e293b" stroke="{color}" stroke-width="3"/>
                <text x="{cx}" y="{cy + 5}" text-anchor="middle" fill="#f8fafc" font-size="12" font-family="monospace">{h["id"]}</text>
            </g>
            """)
        for link in causal_links:
            p = link.get("parent")
            c = link.get("child")
            if p in node_coords and c in node_coords:
                x1, y1 = node_coords[p]
                x2, y2 = node_coords[c]
                svg_elements.append(f"""
                <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#64748b" stroke-width="2" marker-end="url(#arrow)"/>
                <text x="{(x1+x2)/2}" y="{(y1+y2)/2 - 8}" fill="#94a3b8" font-size="10" text-anchor="middle">{link.get("relation", "")}</text>
                """)

    svg_content = f"""
    <svg width="100%" height="260" viewBox="0 0 700 260" xmlns="http://www.w3.org/2000/svg">
        <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="28" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
                <path d="M 0 0 L 10 5 L 0 10 z" fill="#64748b" />
            </marker>
        </defs>
        {"".join(svg_elements)}
    </svg>
    """ if svg_elements else "<p>No causal links defined yet.</p>"

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Deep Digger: {title}</title>
    <style>
        :root {{
            --bg: #0f172a;
            --surface: #1e293b;
            --border: #334155;
            --text: #f8fafc;
            --muted: #94a3b8;
            --primary: #38bdf8;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            margin: 0;
            padding: 24px;
            line-height: 1.5;
        }}
        .container {{ max-width: 1000px; margin: 0 auto; }}
        header {{
            border-bottom: 1px solid var(--border);
            padding-bottom: 16px;
            margin-bottom: 24px;
        }}
        h1 {{ margin: 0 0 8px 0; font-size: 26px; color: var(--primary); }}
        .meta {{ color: var(--muted); font-size: 14px; }}
        .badge {{
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 700;
            color: #fff;
            text-transform: uppercase;
        }}
        .card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 16px;
            margin-bottom: 16px;
        }}
        .card-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            padding-bottom: 8px;
            margin-bottom: 12px;
        }}
        .card-header h3 {{ margin: 0; font-size: 18px; }}
        code {{
            background: #090d16;
            padding: 2px 6px;
            border-radius: 4px;
            font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
            font-size: 13px;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 12px;
            background: var(--surface);
            border-radius: 8px;
            overflow: hidden;
            border: 1px solid var(--border);
        }}
        th, td {{
            padding: 10px 14px;
            text-align: left;
            border-bottom: 1px solid var(--border);
            font-size: 14px;
        }}
        th {{ background: #162032; color: var(--muted); font-weight: 600; }}
        ul {{ margin: 4px 0; padding-left: 20px; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>Deep Digger Diagnostic Dossier</h1>
            <div class="meta">
                <strong>Case:</strong> {title} | <strong>ID:</strong> {case_id} | <strong>Symptom:</strong> {symptom}
            </div>
        </header>

        <h2>Hypotheses & Causal Status</h2>
        {"".join(cards_html) if cards_html else "<p>No hypotheses registered.</p>"}

        <h2>Causal Graph (DAG)</h2>
        <div class="card">{svg_content}</div>

        <h2>Counterfactual Experiment Receipts</h2>
        <table>
            <thead>
                <tr>
                    <th>Experiment ID</th>
                    <th>Hypothesis</th>
                    <th>Phase</th>
                    <th>Verdicts</th>
                    <th>Timestamp (UTC)</th>
                </tr>
            </thead>
            <tbody>
                {"".join(exp_rows) if exp_rows else "<tr><td colspan='5'>No experiments recorded yet.</td></tr>"}
            </tbody>
        </table>

        <h2>Tamper-Evident Evidence Ledger</h2>
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Hypothesis</th>
                    <th>Effect</th>
                    <th>Kind</th>
                    <th>Note</th>
                    <th>SHA-256</th>
                </tr>
            </thead>
            <tbody>
                {"".join(ev_rows) if ev_rows else "<tr><td colspan='6'>No evidence attached yet.</td></tr>"}
            </tbody>
        </table>
    </div>
</body>
</html>
"""


def _comment_safe(value: object) -> str:
    text = " ".join(str(value if value is not None else "").split())
    return text.replace("\\", "\\\\").replace('"""', "'''").replace("*/", "* /")


def cmd_repro(args) -> None:
    data = read_case(args.folder)
    folder = Path(args.folder)
    ext = "py" if args.template == "python" else "js" if args.template == "node" else "sh"
    repro_file = folder / f"repro_{args.template}.{ext}"
    if repro_file.exists():
        raise ValueError(f"reproducer file already exists: {repro_file}")

    case_id, title, symptom = (_comment_safe(data.get(k, "")) for k in ("id", "title", "symptom"))

    if args.template == "python":
        content = f'''#!/usr/bin/env python3
"""Deep Digger Isolated Reproducer
Case ID: {case_id}
Title: {title}
Symptom: {symptom}
"""
import sys

def run_reproduction():
    # Insert minimal reproduction code triggering the defect
    raise NotImplementedError("Implement minimal failure condition")

if __name__ == "__main__":
    try:
        run_reproduction()
        sys.exit(0) # PASS
    except Exception as exc:
        print(f"Reproduction triggered: {{exc}}", file=sys.stderr)
        sys.exit(1) # FAIL
'''
    elif args.template == "node":
        content = f'''#!/usr/bin/env node
/**
 * Deep Digger Isolated Reproducer
 * Case ID: {case_id}
 * Title: {title}
 * Symptom: {symptom}
 */

function runReproduction() {{
  // Insert minimal reproduction code triggering the defect
  throw new Error("Implement minimal failure condition");
}}

try {{
  runReproduction();
  process.exit(0); // PASS
}} catch (err) {{
  console.error("Reproduction triggered:", err.message);
  process.exit(1); // FAIL
}}
'''
    else:
        content = f'''#!/usr/bin/env bash
# Deep Digger Isolated Reproducer
# Case ID: {case_id}
# Title: {title}
# Symptom: {symptom}
set -euo pipefail

echo "Running reproduction..."
exit 1
'''
    repro_file.write_text(content, encoding="utf-8")
    if args.template == "bash":
        os.chmod(repro_file, 0o755)
    print(json.dumps({"reproducer": str(repro_file), "template": args.template}))


def cmd_bisect(args) -> None:
    if not args.allow_exec:
        raise ValueError("bisect execution requires --allow-exec")
    validate_timeout(args.timeout)
    data = read_case(args.folder)
    require_hypothesis(data, args.hypothesis)
    argv = normalize_command(args.command)

    try:
        subprocess.run(["git", "rev-parse", "--is-inside-work-tree"], check=True, capture_output=True, timeout=60)
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise ValueError("current directory is not inside a git repository") from None

    start_cmd = ["git", "bisect", "start", args.bad, args.good]
    p_start = subprocess.run(start_cmd, capture_output=True, text=True, timeout=60)
    if p_start.returncode != 0:
        raise ValueError(f"git bisect start failed: {p_start.stderr.strip()}")

    output_text = ""
    try:
        p_run = subprocess.run(["git", "bisect", "run", *argv], capture_output=True, text=True, timeout=args.timeout)
        output_text = p_run.stdout + "\n" + p_run.stderr
    except subprocess.TimeoutExpired:
        output_text = ""
    finally:
        subprocess.run(["git", "bisect", "reset"], capture_output=True, timeout=60)

    culprit = None
    for line in output_text.splitlines():
        if "is the first bad commit" in line:
            culprit = line.split()[0]
            break
    result = {
        "hypothesis": args.hypothesis,
        "good_revision": args.good,
        "bad_revision": args.bad,
        "culprit_commit": culprit,
        "time": utc_now()
    }
    record = {
        "id": f"E-{len(data['evidence']) + 1}",
        "hypothesis": args.hypothesis,
        "kind": "observation",
        "effect": "supports" if culprit else "neutral",
        "note": f"Git bisect: first bad commit is {culprit or 'unresolved'}",
        "source": None,
        "sha256": None,
        "created_at": utc_now(),
        "bisect_metadata": result
    }
    data["evidence"].append(record)
    save_case(args.folder, data)
    print(json.dumps(result, ensure_ascii=False, indent=2))


def cmd_report(args) -> None:
    data = read_case(args.folder)
    result = {"title": data["title"], "symptom": data["symptom"], "case_id": data["id"],
              "hypotheses": [{**h, **classify_case(data, h["id"])} for h in data["hypotheses"]],
              "causal_links": data.get("causal_links", []),
              "experiment_count": len(data["experiments"]),
              "evidence_count": len(data["evidence"]),
              "evidence": data["evidence"], "experiments": data["experiments"]}
    if getattr(args, "html", None):
        html_doc = generate_html_report(result)
        out_path = Path(args.html)
        out_path.write_text(html_doc, encoding="utf-8")
        print(json.dumps({"html_report": str(out_path), "status": "generated"}))
        return
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
    if data.get("causal_links"):
        print("## Causal Dependency Graph\n")
        for link in data["causal_links"]:
            print(f"- {link['parent']} --[{link['relation']}]--> {link['child']}")
        print()
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


def render_terminal_dashboard(data: dict, no_color: bool = False) -> str:
    def col(text: str, code: str) -> str:
        return f"\033[{code}m{text}\033[0m" if not no_color else text

    BOLD = "1"
    CYAN = "1;36"
    GREEN = "1;32"
    YELLOW = "1;33"
    RED = "1;31"
    MAGENTA = "1;35"
    BLUE = "1;34"
    DIM = "2"

    width = 76
    lines = []

    # Banner
    lines.append(col("┌" + "─" * (width - 2) + "┐", CYAN))
    title_str = "DEEP DIGGER · INCIDENT INVESTIGATION DOSSIER"
    subtitle_str = "Catch That Bug · Evidence-Gated Root Cause & Falsification"
    lines.append(col(f"│{title_str:^{width-2}}│", CYAN))
    lines.append(col(f"│{subtitle_str:^{width-2}}│", DIM))
    lines.append(col("└" + "─" * (width - 2) + "┘", CYAN))
    lines.append("")

    # Incident Overview
    lines.append(col("  INCIDENT OVERVIEW", BOLD))
    lines.append(f"  {col('Case ID:', DIM)}      {data.get('id', 'N/A')}")
    lines.append(f"  {col('Title:', DIM)}        {col(data.get('title', 'N/A'), BOLD)}")
    lines.append(f"  {col('Symptom:', DIM)}      {data.get('symptom', 'N/A')}")
    lines.append(f"  {col('Created:', DIM)}      {data.get('created_at', 'N/A')}")
    counts_str = f"{len(data.get('hypotheses', []))} Hypotheses  │  {len(data.get('experiments', []))} Experiments  │  {len(data.get('evidence', []))} Evidence Records"
    lines.append(f"  {col('Metrics:', DIM)}      {col(counts_str, BLUE)}")
    lines.append("")

    # Hypotheses Matrix
    lines.append(col("┌─ HYPOTHESES & CONFIDENCE MATRIX " + "─" * (width - 34) + "┐", BOLD))
    hypotheses = data.get("hypotheses", [])
    if not hypotheses:
        lines.append(f"│ {col('No hypotheses registered yet.', DIM)}")
    for idx, h in enumerate(hypotheses):
        state = classify_case(data, h["id"])
        status = state["status"]
        status_col = GREEN if status == "experimental-pattern" else (YELLOW if status == "reproduced" else RED)
        status_badge = col(f"[{status.upper()}]", status_col)

        lines.append(f"│ {col(h['id'], BOLD)} {status_badge} {col(h['claim'], BOLD)}")
        lines.append(f"│   {col('Test:', DIM)}        {h.get('discriminating_test', 'N/A')}")

        reasons = state.get("missing_gates", []) + state.get("conflicts", [])
        if not reasons:
            lines.append(f"│   {col('Gates:', DIM)}       {col('✓ All causal gates satisfied (baseline, intervention, reversal, control)', GREEN)}")
        else:
            lines.append(f"│   {col('Missing:', DIM)}     {col(', '.join(reasons), YELLOW)}")

        sup = state.get("supporting_evidence", [])
        ref = state.get("refuting_evidence", [])
        ev_summary = f"{len(sup)} Supporting ({', '.join(sup) if sup else 'none'})  │  {len(ref)} Refuting ({', '.join(ref) if ref else 'none'})"
        lines.append(f"│   {col('Evidence:', DIM)}    {ev_summary}")
        if idx < len(hypotheses) - 1:
            lines.append("├" + "─" * (width - 2) + "┤")
    lines.append("└" + "─" * (width - 2) + "┘")
    lines.append("")

    # Causal Dependency Graph
    links = data.get("causal_links", [])
    if links:
        lines.append(col("┌─ CAUSAL DEPENDENCY GRAPH (DAG) " + "─" * (width - 33) + "┐", BOLD))
        for l in links:
            parent = l.get("parent", "")
            child = l.get("child", "")
            rel = l.get("relation", "causes")
            arrow = f"─────({rel})────►"
            lines.append(f"│  {col(parent, CYAN)} {col(arrow, MAGENTA)} {col(child, CYAN)}")
        lines.append("└" + "─" * (width - 2) + "┘")
        lines.append("")

    # Evidence Ledger
    evidence_list = data.get("evidence", [])
    if evidence_list:
        lines.append(col("┌─ EVIDENCE LEDGER " + "─" * (width - 19) + "┐", BOLD))
        for ev in evidence_list:
            sha = ev.get("sha256") or "N/A"
            sha_short = sha[:8] + ".." if len(sha) > 8 else sha
            effect_col = GREEN if ev.get("effect") == "supports" else (RED if ev.get("effect") == "refutes" else DIM)
            effect_str = col(f"[{ev.get('effect', 'neutral').upper()}]", effect_col)
            source_str = f" ({ev['source']})" if ev.get("source") else ""
            lines.append(f"│ {col(ev.get('id', 'E-?'), BOLD)} {effect_str} {col(sha_short, BLUE)}{source_str}")
            lines.append(f"│   {col('Note:', DIM)} {ev.get('note', '')}")
        lines.append("└" + "─" * (width - 2) + "┘")
        lines.append("")

    # Recommended Next Action
    lines.append(col("┌─ RECOMMENDED NEXT ACTION " + "─" * (width - 27) + "┐", YELLOW))
    if not hypotheses:
        lines.append(f"│ ▶ Register initial falsifiable hypotheses: {col('deep-digger hypothesis <folder> ...', CYAN)}")
    else:
        for h in hypotheses:
            state = classify_case(data, h["id"])
            if state.get("conflicts"):
                action = f"Investigate contradictory evidence for {h['id']} before any causal assertion"
            elif state.get("missing_gates"):
                action = f"Execute safe discriminating experiment for {h['id']}: missing {state['missing_gates'][0]}"
            else:
                action = f"All verification gates met for {h['id']}; review confounders & alternatives"
            lines.append(f"│ ▶ {col(h['id'], BOLD)}: {col(action, YELLOW)}")
    lines.append("└" + "─" * (width - 2) + "┘")

    return "\n".join(lines)


def cmd_view(args) -> None:
    data = read_case(args.folder)
    no_color = getattr(args, "no_color", False) or bool(os.environ.get("NO_COLOR")) or not sys.stdout.isatty()
    output = render_terminal_dashboard(data, no_color=no_color)
    print(output)


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
    validate_timeout(args.timeout)
    input_path, output_path = Path(args.input), Path(args.output)
    if input_path.resolve() == output_path.resolve():
        raise ValueError("output must not overwrite input")
    if not input_path.is_file() or output_path.exists():
        raise ValueError("input must be a file and output must not exist")
    if not output_path.parent.is_dir():
        raise ValueError("output directory must exist")
    argv = normalize_command(args.command)
    if not any("{input}" in item for item in argv):
        raise ValueError("command must include {input} placeholder")
    
    granularity = getattr(args, "granularity", "lines")
    if granularity == "tokens":
        text = input_path.read_text(encoding="utf-8")
        chunks = re.findall(r"\w+|\s+|[^\w\s]", text)
    else:
        # newline='' preserves line terminators on all platforms.
        with input_path.open("r", encoding="utf-8", newline="") as stream:
            chunks = stream.readlines()
    with tempfile.TemporaryDirectory(prefix="deep-digger-min-") as temporary:
        candidate_path = Path(temporary) / "candidate.txt"

        def oracle(candidate: list[str]) -> str:
            with candidate_path.open("w", encoding="utf-8", newline="") as stream:
                stream.write("".join(candidate))
            invocation = [item.replace("{input}", str(candidate_path)) for item in argv]
            outcomes = [run_command(invocation, args.timeout)["verdict"] for _ in range(args.repeats)]
            return outcomes[0] if len(set(outcomes)) == 1 else "UNRESOLVED"

        minimized, complete = ddmin(chunks, oracle, args.max_tests)
    with output_path.open("x", encoding="utf-8", newline="") as stream:
        stream.write("".join(minimized))
    print(json.dumps({"output": str(output_path), "original_units": len(chunks),
                      "retained_units": len(minimized), "granularity": granularity,
                      "one_minimal_verified": complete,
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
    rep.add_argument("--html", help="Path to generate standalone HTML report")
    rep.set_defaults(func=cmd_report)
    view = sub.add_parser("view", help="Render interactive ANSI terminal dashboard for case ledger")
    view.add_argument("folder")
    view.add_argument("--no-color", action="store_true", help="Disable ANSI color codes")
    view.set_defaults(func=cmd_view)
    nxt = sub.add_parser("next", help="Return next discriminating or verification step")
    nxt.add_argument("folder"); nxt.set_defaults(func=cmd_next)
    audit = sub.add_parser("audit", help="Check current source hashes against captured evidence")
    audit.add_argument("folder"); audit.set_defaults(func=cmd_audit)
    repro = sub.add_parser("repro", help="Generate an isolated reproduction harness template")
    repro.add_argument("folder")
    repro.add_argument("--template", choices=("python", "node", "bash"), default="python")
    repro.set_defaults(func=cmd_repro)
    bis = sub.add_parser("bisect", help="Run automated git bisect and record regression culprit into ledger")
    bis.add_argument("folder")
    bis.add_argument("--hypothesis", required=True)
    bis.add_argument("--good", required=True)
    bis.add_argument("--bad", required=True)
    bis.add_argument("--timeout", type=float, default=60)
    bis.add_argument("--allow-exec", action="store_true")
    bis.set_defaults(func=cmd_bisect)
    mini = sub.add_parser("minimize", help="Reduce a stable failing text input using an oracle")
    mini.add_argument("--input", required=True); mini.add_argument("--output", required=True)
    mini.add_argument("--granularity", choices=("lines", "tokens"), default="lines")
    mini.add_argument("--repeats", type=int, default=3); mini.add_argument("--max-tests", type=int, default=200)
    mini.add_argument("--timeout", type=float, default=15)
    mini.add_argument("--allow-exec", action="store_true")
    mini.set_defaults(func=cmd_minimize)
    trc = sub.add_parser("trace", help="Ingest OpenTelemetry or JSON trace and attach diagnostic evidence")
    trc.add_argument("folder")
    trc.add_argument("--hypothesis", required=True)
    trc.add_argument("--file", required=True)
    trc.set_defaults(func=cmd_trace)
    lnk = sub.add_parser("link", help="Model causal relations between hypotheses (DAG)")
    lnk.add_argument("folder")
    lnk.add_argument("--parent", required=True)
    lnk.add_argument("--child", required=True)
    lnk.add_argument("--relation", choices=("causes", "triggers", "enables"), default="causes")
    lnk.set_defaults(func=cmd_link)
    flk = sub.add_parser("flake", help="Statistical repetition oracle to detect flakiness and measure P(FAIL)")
    flk.add_argument("folder")
    flk.add_argument("--hypothesis", required=True)
    flk.add_argument("--repeats", type=int, default=10)
    flk.add_argument("--timeout", type=float, default=15)
    flk.add_argument("--allow-exec", action="store_true")
    flk.set_defaults(func=cmd_flake)
    return parser


def main(argv: list[str] | None = None) -> int:
    try:
        original = list(sys.argv[1:] if argv is None else argv)
        if original and original[0] in {"experiment", "minimize", "flake", "bisect"}:
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
