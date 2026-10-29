"""Safe deterministic fixture: FAIL iff both markers are present."""
from pathlib import Path
import sys


def main() -> int:
    content = Path(sys.argv[1]).read_text(encoding="utf-8")
    lines = set(content.splitlines())
    return 1 if {"BUG", "TRIGGER"}.issubset(lines) else 0


if __name__ == "__main__":
    raise SystemExit(main())
