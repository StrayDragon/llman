#!/usr/bin/env python3
"""Check .agents/skills/*/SKILL.md version metadata matches Cargo.toml workspace version."""
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def extract_workspace_version() -> str:
    cargo_toml = REPO_ROOT / "Cargo.toml"
    content = cargo_toml.read_text(encoding="utf-8")
    for line in content.splitlines():
        line = line.strip()
        if line.startswith("version ") and "workspace" not in line:
            # Match: version = "X.Y.Z"
            m = re.search(r'"([^"]+)"', line)
            if m:
                return m.group(1)
    raise SystemExit(f"Cannot find workspace version in {cargo_toml}")


def main() -> int:
    expected = extract_workspace_version()
    skills_dir = REPO_ROOT / ".agents" / "skills"
    if not skills_dir.is_dir():
        return 0  # no skills yet, nothing to check

    errors = []
    for skill_dir in sorted(skills_dir.iterdir()):
        if not skill_dir.is_dir() or skill_dir.name.startswith("."):
            continue
        skill_md = skill_dir / "SKILL.md"
        if not skill_md.is_file():
            continue
        content = skill_md.read_text(encoding="utf-8")
        # Find version in YAML frontmatter
        m = re.search(r'^  version:\s*"([^"]+)"', content, re.MULTILINE)
        if m:
            if m.group(1) != expected:
                print(
                    f"{skill_md}: version mismatch: "
                    f'"{m.group(1)}" != expected "{expected}"'
                )
                errors.append(str(skill_md))
        else:
            print(f"{skill_md}: no version metadata found in frontmatter")
            errors.append(str(skill_md))

    if errors:
        print(f"\nFix: update version to '{expected}' in the files above and retry.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
