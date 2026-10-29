#!/usr/bin/env python3
"""
OpenReview Orchestrator CLI helper
Classifies intent, builds execution DAGs, and verifies system readiness.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from typing import Dict, List, Optional


OPERATING_MODES = ["REVIEW", "FIX", "INVESTIGATE", "VERIFY_CONFIG"]


def classify_intent(query: str) -> str:
    """Classify natural language user or PR intent into an orchestrator mode."""
    q = query.lower()
    
    if any(k in q for k in ["setup", "configure", "config", "audit invariant", "verify-config"]):
        return "VERIFY_CONFIG"
    if any(k in q for k in ["why did", "flaky", "regression", "minimize", "ddmin", "root cause", "isolate"]):
        return "INVESTIGATE"
    if any(k in q for k in ["fix", "remediate", "repair", "resolve error", "patch"]):
        return "FIX"
    # Default to read-only Review
    return "REVIEW"


def build_dag(mode: str, repo: Optional[str] = None, pr: Optional[int] = None) -> Dict[str, object]:
    """Generate the Directed Acyclic Graph (DAG) for the given mode."""
    if mode == "REVIEW":
        return {
            "mode": "REVIEW",
            "steps": [
                {"step": 1, "action": "fetch_diff", "engine": "github_api", "read_only": True},
                {"step": 2, "action": "inspect_ast", "engine": "openreview_core", "read_only": True},
                {"step": 3, "action": "classify_findings", "standard": "agency-code-reviewer_3_tier"},
                {"step": 4, "action": "format_suggestions", "template": "github_suggestion_block"}
            ],
            "sandbox": "read_only",
            "publish_permission": "read"
        }
    elif mode == "FIX":
        return {
            "mode": "FIX",
            "steps": [
                {"step": 1, "action": "check_authorization", "required_roles": ["write", "maintain", "admin"]},
                {"step": 2, "action": "provision_sandbox", "engine": "docker_or_local_ephemeral"},
                {"step": 3, "action": "apply_minimal_patch", "principle": "laziness_protocol"},
                {"step": 4, "action": "verification_gate", "runners": ["cargo", "bun", "pytest", "go", "npm"]},
                {"step": 5, "action": "attributable_commit_and_push", "gate_condition": "tests_passed_strictly"}
            ],
            "sandbox": "writable_ephemeral",
            "publish_permission": "write"
        }
    elif mode == "INVESTIGATE":
        return {
            "mode": "INVESTIGATE",
            "steps": [
                {"step": 1, "action": "formulate_hypothesis", "tool": "deep_digger_ledger"},
                {"step": 2, "action": "define_bounded_oracle", "exit_codes": {"0": "PASS", "1": "FAIL"}},
                {"step": 3, "action": "run_ddmin", "tool": "deep-digger-min", "algorithm": "zeller_ddmin"},
                {"step": 4, "action": "record_evidence", "tool": "deep_digger_ledger"},
                {"step": 5, "action": "handoff_minimal_test", "target": "openreview_fix"}
            ],
            "sandbox": "bounded_oracle_execution"
        }
    else:  # VERIFY_CONFIG
        return {
            "mode": "VERIFY_CONFIG",
            "steps": [
                {"step": 1, "action": "validate_environment_keys", "tool": "openreview verify-config"},
                {"step": 2, "action": "audit_system_invariants", "script": "verify_openreview.py"},
                {"step": 3, "action": "generate_audit_report"}
            ],
            "sandbox": "host_inspection"
        }


def main() -> int:
    parser = argparse.ArgumentParser(description="OpenReview Master Orchestrator CLI")
    subparsers = parser.add_subparsers(dest="subcommand")

    route_cmd = subparsers.add_parser("route", help="Route natural language query to execution DAG")
    route_cmd.add_argument("query", help="User intent or command string")
    route_cmd.add_argument("--repo", help="Target repository (owner/name)")
    route_cmd.add_argument("--pr", type=int, help="Target PR number")

    plan_cmd = subparsers.add_parser("plan", help="Generate JSON execution plan for an explicit mode")
    plan_cmd.add_argument("mode", choices=OPERATING_MODES, help="Operating mode")
    plan_cmd.add_argument("--repo", help="Target repository (owner/name)")
    plan_cmd.add_argument("--pr", type=int, help="Target PR number")

    args = parser.parse_args()

    if args.subcommand == "route":
        mode = classify_intent(args.query)
        dag = build_dag(mode, args.repo, args.pr)
        print(json.dumps({"intent": args.query, "classified_mode": mode, "dag": dag}, indent=2))
        return 0
    elif args.subcommand == "plan":
        dag = build_dag(args.mode, args.repo, args.pr)
        print(json.dumps(dag, indent=2))
        return 0
    else:
        parser.print_help()
        return 1


if __name__ == "__main__":
    sys.exit(main())
