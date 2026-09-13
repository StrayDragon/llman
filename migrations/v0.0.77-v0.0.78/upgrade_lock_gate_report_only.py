#!/usr/bin/env python3
"""One-shot upgrade: lock-gate report-only (v0.0.77 → v0.0.78).

Removes the removed ack metadata fields from `llmanspec/changes/**/proposal.md`
(ACTIVE changes only; `changes/archive/` is skipped — history stays read-only):

  rules_touched  -> remove (block list, flow list, or scalar)
  agent_acked    -> remove (same shapes)

Lock-gate semantics moved to report-only (spec-format r135 / S0): locked-rule
edits are WARNING-reported and never block; no ack metadata exists anymore.

Default is dry-run (prints what would change). `--apply` performs the writes.
Idempotent: a second run with nothing to do prints `no-op`.
Usage: python3 upgrade_lock_gate_report_only.py [--apply] [--root <path>]
"""

import argparse
import re
import sys
from pathlib import Path

REMOVED_FIELDS = {"rules_touched", "agent_acked"}


def find_proposals(root: Path):
    changes = root / "llmanspec" / "changes"
    if not changes.is_dir():
        return []
    return [
        p
        for p in sorted(changes.rglob("proposal.md"))
        if "archive" not in p.relative_to(changes).parts
    ]


def parse_frontmatter(text: str):
    if not text.startswith("---\n"):
        return None, text
    m = re.match(r"^---\n(.*?)\n---\n?", text, re.DOTALL)
    if not m:
        return None, text
    return m.group(1), text[m.end():]


def strip_fields(fm: str) -> tuple[str, list[str]]:
    """Drop removed fields (key line + any more-indented list/scalar lines)."""
    lines = fm.splitlines(keepends=True)
    out: list[str] = []
    removed: list[str] = []
    skipping = False
    for line in lines:
        key_match = re.match(r"^([A-Za-z_][A-Za-z0-9_]*):", line)
        if key_match:
            name = key_match.group(1)
            if name in REMOVED_FIELDS:
                removed.append(name)
                skipping = True
                # inline flow style (`key: [a, b]`) or scalar on the same line
                if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*:\s*$", line):
                    skipping = False
                continue
            skipping = False
        if skipping:
            if re.match(r"^\s", line) or line.strip().startswith("-"):
                continue
            skipping = False
        out.append(line)
    return "".join(out), removed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--apply", action="store_true", help="write changes (default: dry-run)")
    ap.add_argument("--root", default=".", help="project root containing llmanspec/")
    args = ap.parse_args()

    root = Path(args.root)
    proposals = find_proposals(root)
    if not proposals:
        print("no active change proposals found; no-op")
        return 0

    touched = 0
    for path in proposals:
        text = path.read_text(encoding="utf-8")
        fm, body = parse_frontmatter(text)
        if fm is None:
            continue
        new_fm, removed = strip_fields(fm)
        if not removed:
            continue
        new_text = f"---\n{new_fm.rstrip()}\n---\n{body}"
        rel = path.relative_to(root)
        print(f"{rel}: removed {', '.join(removed)}")
        touched += 1
        if args.apply:
            path.write_text(new_text, encoding="utf-8")

    if touched == 0:
        print("no-op")
        return 0
    print(f"\n{'applied' if args.apply else 'dry-run'}: {touched} proposal(s) affected")
    if not args.apply:
        print("re-run with --apply to write")
    print("next: llman sdd validate --all --strict --no-check")
    return 0


if __name__ == "__main__":
    sys.exit(main())
