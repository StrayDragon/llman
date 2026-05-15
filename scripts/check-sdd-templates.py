#!/usr/bin/env python3
from pathlib import Path
import re
import sys
from typing import Dict, List, Optional

SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FORBIDDEN_PROMPT_TOOLING_RE = re.compile(
    r"(?i)/llman-sdd:|\bclaude\b|\bcodex\b|slash commands?"
)


def is_skill_template(path: Path) -> bool:
    return path.parent.name == "skills" and path.name.startswith("llman-sdd-")


def parse_frontmatter(
    path: Path, lines: List[str], errors: List[str]
) -> Optional[Dict[str, str]]:
    if not lines or lines[0].strip() != "---":
        return None

    end_idx = None
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end_idx = i
            break
    if end_idx is None:
        errors.append(f"{path}: unterminated frontmatter")
        return None

    data: Dict[str, str] = {}
    for line in lines[1:end_idx]:
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        key = key.strip()
        value = value.strip()
        if not key:
            continue
        value = value.strip('"').strip("'")
        data[key] = value
    return data


def validate_skill_frontmatter(path: Path, lines: List[str], errors: List[str]) -> None:
    if not is_skill_template(path):
        return

    frontmatter = parse_frontmatter(path, lines, errors)
    if frontmatter is None:
        errors.append(f"{path}: skill template missing YAML frontmatter")
        return

    name = frontmatter.get("name", "").strip()
    description = frontmatter.get("description", "").strip()
    if not name:
        errors.append(f"{path}: frontmatter missing name")
    else:
        if len(name) > 64:
            errors.append(f"{path}: name exceeds 64 characters")
        if not SKILL_NAME_RE.match(name):
            errors.append(f"{path}: name must be lowercase alphanumeric with hyphens")
        if name != path.stem:
            errors.append(f"{path}: name must match file stem '{path.stem}'")
    if not description:
        errors.append(f"{path}: frontmatter missing description")
    elif len(description) > 1024:
        errors.append(f"{path}: description exceeds 1024 characters")


def collect_template_files(locale_dir: Path, errors: List[str]) -> List[str]:
    files: List[str] = []
    for path in sorted(locale_dir.rglob("*.md")):
        rel = path.relative_to(locale_dir).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        content = "\n".join(lines)
        forbidden_match = FORBIDDEN_PROMPT_TOOLING_RE.search(content)
        if forbidden_match:
            errors.append(
                f"{path}: forbidden tool-specific prompt content detected: '{forbidden_match.group(0)}'"
            )
        validate_skill_frontmatter(path, lines, errors)
        files.append(rel)
    if not files:
        errors.append(f"{locale_dir}: no markdown templates found")
    return files


def validate_markdown_root(templates_root: Path, errors: List[str]) -> List[str]:
    if not templates_root.exists():
        errors.append(f"ERROR: {templates_root} not found")
        return []

    locale_dirs = sorted([p for p in templates_root.iterdir() if p.is_dir()])
    if not locale_dirs:
        errors.append(f"ERROR: no locale directories found under {templates_root}")
        return []

    locales = [p.name for p in locale_dirs]
    base_locale = "en" if (templates_root / "en").is_dir() else locales[0]
    base_dir = templates_root / base_locale
    base_files = collect_template_files(base_dir, errors)

    for locale_dir in locale_dirs:
        if locale_dir == base_dir:
            continue
        files = collect_template_files(locale_dir, errors)

        base_set = set(base_files)
        other_set = set(files)

        for rel in sorted(base_set - other_set):
            errors.append(f"{locale_dir / rel}: missing template (expected {rel})")
        for rel in sorted(other_set - base_set):
            errors.append(f"{locale_dir / rel}: extra template (not in {base_locale})")

    return locales


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    templates_root = repo_root / "templates"
    errors: List[str] = []
    sdd_root = templates_root / "sdd"

    sdd_locales = validate_markdown_root(sdd_root, errors)

    if errors:
        print("SDD template checks failed:")
        for err in errors:
            print(f"- {err}")
        return 1

    locale_list = ", ".join(sdd_locales)
    print(f"SDD template checks passed for locales: {locale_list}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
