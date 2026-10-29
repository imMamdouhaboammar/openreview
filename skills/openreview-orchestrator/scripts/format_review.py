#!/usr/bin/env python3
"""
Review comment formatter and validator for OpenReview.
Validates 3-tier severity tags and GitHub ```suggestion``` blocks.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from typing import Dict, List, Optional


SEVERITY_BADGES = {
    "BLOCKER": "🔴",
    "SUGGESTION": "🟡",
    "NIT": "💭"
}


def format_finding(
    severity: str,
    category: str,
    title: str,
    file_path: str,
    line: int,
    why: str,
    suggestion: Optional[str] = None
) -> str:
    """Format a single finding into the canonical OpenReview markdown structure."""
    sev_key = severity.upper()
    badge = SEVERITY_BADGES.get(sev_key, "🟡")
    
    lines = [
        f"{badge} **{category}: {title}**",
        f"Line {line} in `{file_path}`: {why.splitlines()[0] if why else ''}",
        "",
        f"**Why:** {why.strip()}"
    ]

    if suggestion is not None:
        clean_code = suggestion.strip("\r\n")
        lines.extend([
            "",
            "**Suggestion:**",
            "```suggestion",
            clean_code,
            "```"
        ])

    return "\n".join(lines)


def validate_review_markdown(content: str) -> List[str]:
    """Validate that review content conforms to formatting rules."""
    errors = []
    
    # Check for presence of at least one valid severity badge
    has_badge = any(b in content for b in SEVERITY_BADGES.values())
    if not has_badge:
        errors.append("Missing required severity badge (🔴, 🟡, or 💭)")

    # Check suggestion block syntax
    open_suggestions = len(re.findall(r"^```suggestion\s*$", content, re.MULTILINE))
    close_blocks = len(re.findall(r"^```\s*$", content, re.MULTILINE))

    if open_suggestions > 0 and close_blocks < open_suggestions:
        errors.append(f"Unclosed suggestion block: found {open_suggestions} open blocks")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="OpenReview comment formatter and validator")
    parser.add_argument("--validate", help="Validate a markdown file containing review comments")
    args = parser.parse_args()

    if args.validate:
        with open(args.validate, "r", encoding="utf-8") as f:
            content = f.read()
        errs = validate_review_markdown(content)
        if errs:
            for e in errs:
                print(f"[FAIL] {e}", file=sys.stderr)
            return 1
        print("[OK] Review markdown meets format standards.")
        return 0
    
    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
