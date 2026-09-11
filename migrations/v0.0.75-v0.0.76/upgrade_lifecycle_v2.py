#!/usr/bin/env python3
"""One-shot upgrade: llman SDD lifecycle v2 (v0.0.75 → v0.0.76).

Removes legacy proposal frontmatter fields under `llmanspec/changes/*/proposal.md`
(ACTIVE changes only; `changes/archive/` is skipped — history stays read-only):

  skip_specs_landing: true   -> remove field, write `needs_specs_change: false`
  skip_specs_landing: false  -> remove field
  checkpointed / checkpoint_sha / checkpointSha -> remove
  baseSha                     -> rename to `base_sha` (refuse on conflict)
  rules_edit_acked            -> remove; if truthy and no rules_touched,
                                 print a MANUAL list (cannot guess req-ids)

Default is dry-run (prints what would change). `--apply` performs the writes.
Idempotent: a second run with nothing to do prints `no-op`.
Usage: python3 upgrade_lifecycle_v2.py [--apply] [--root <path>]
"""

import argparse
import os
import re
import sys
from pathlib import Path

LEGACY_FIELDS = {"skip_specs_landing", "checkpointed", "checkpoint_sha", "checkpointSha", "rules_edit_acked"}
ALIAS_FIELD = "baseSha"
NEW_FIELD = "needs_specs_change"


def yaml_bool(value: str) -> bool | None:
    """Interpret a YAML scalar as bool; None when not a bool-like."""
    v = value.strip().lower()
    if v in ("true", "yes", "1"):
        return True
    if v in ("false", "no", "0"):
        return False
    return None


def parse_frontmatter(text: str):
    """Return (frontmatter block, body) or (None, text) when no frontmatter."""
    if not text.startswith("---\n"):
        return None, text
    m = re.match(r"^---\n(.*?)\n---\n?", text, re.DOTALL)
    if not m:
        return None, text
    return m.group(1), text[m.end():]


def rewrite_one(text: str) -> dict:
    """Compute the rewrite of one proposal.md. Returns {status, new_text, notes}."""
    fm, body = parse_frontmatter(text)
    if fm is None:
        return {"status": "noop", "new_text": text, "notes": []}
    lines = fm.splitlines()
    out: list[str] = []
    changed = False
    notes: list[str] = []

    skip_value: bool | None = None
    # Pass 1: capture skip_specs_landing (needs rewrite AFTER removal so the
    # field order is stable), drop other legacy fields / alias.
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("#") or ":" not in stripped:
            if stripped not in ("",) or line == "":
                out.append(line)
            continue
        key, _, value = stripped.partition(":")
        key = key.strip()
        value = value.strip()
        if key == "skip_specs_landing":
            changed = True
            skip_value = yaml_bool(value)
            if skip_value is True:
                notes.append("skip_specs_landing: true -> needs_specs_change: false")
            else:
                notes.append("skip_specs_landing removed (false default)")
            continue  # dropped here; needs_specs_change appended below
        if key in LEGACY_FIELDS:
            changed = True
            if key == "rules_edit_acked" and yaml_bool(value) is True:
                notes.append("MANUAL: rules_edit_acked: true — determine the edited "
                             "req-ids and write `rules_touched: [...]` yourself")
            else:
                notes.append(f"{key} removed")
            continue
        if key == ALIAS_FIELD:
            conflict = any(l.strip().startswith("base_sha:") for l in out)
            if conflict:
                notes.append(f"MANUAL: baseSha renamed to base_sha but base_sha already present "
                             f"({value!r}) — resolve manually")
                out.append(line)
            else:
                changed = True
                notes.append("baseSha -> base_sha")
                out.append(f"base_sha: {value}")
            continue
        out.append(line)

    if skip_value is True:
        out.append(f"{NEW_FIELD}: false")

    new_fm = "\n".join(out)
    new_text = f"---\n{new_fm}\n---\n\n{body}" if fm != "" else text
    if not changed:
        return {"status": "noop", "new_text": text, "notes": []}
    return {"status": "rewrite", "new_text": new_text, "notes": notes}


def main() -> int:
    ap = argparse.ArgumentParser(description="llman SDD lifecycle v2 one-shot upgrade")
    ap.add_argument("--apply", action="store_true", help="write changes (default: dry-run)")
    ap.add_argument("--root", default=".", help="project root (default: current dir)")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    changes = root / "llmanspec" / "changes"
    if not changes.is_dir():
        print(f"error: no {changes} directory; is {root} an llmanspec project?")
        return 1

    plans: list[dict] = []
    for change_dir in sorted(changes.iterdir()):
        if not change_dir.is_dir() or change_dir.name == "archive":
            continue
        proposal = change_dir / "proposal.md"
        if not proposal.is_file():
            continue
        text = proposal.read_text(encoding="utf-8")
        result = rewrite_one(text)
        if result["status"] == "rewrite":
            plans.append({"id": change_dir.name, "path": proposal, **result})

    if not plans:
        print("no-op: nothing to upgrade")
        return 0

    for plan in plans:
        print(f"{plan['id']}:")
        for note in plan["notes"]:
            marker = "manual:" if note.startswith("MANUAL:") else "auto:  "
            print(f"  {marker} {note}")

    if not args.apply:
        print(f"\n{dry_run_hint(len(plans))}")
        return 0

    for plan in plans:
        plan["path"].write_text(plan["new_text"], encoding="utf-8")
        print(f"  wrote {plan['path'].relative_to(root)}")

    print("\nDone. Next:")
    print("  1. Resolve every `manual:` item above by writing `rules_touched: [...]`.")
    print("  2. Run `llman sdd validate --all --strict --no-check` — must be green.")
    return 0


def dry_run_hint(n: int) -> str:
    return f"{n} file(s) would change. Re-run with --apply to write."


if __name__ == "__main__":
    sys.exit(main())
