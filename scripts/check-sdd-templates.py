#!/usr/bin/env python3
from pathlib import Path
import re
import sys
from typing import Dict, List, Optional

HTML_VERSION_RE = re.compile(
    r"^<!--\s*llman-template-version:\s*([0-9]+)\s*-->\s*$"
)
FRONTMATTER_VERSION_RE = re.compile(
    r"^\s*llman-template-version:\s*([0-9]+)\s*$"
)


def extract_version(path: Path, lines: List[str], errors: List[str]) -> Optional[str]:
    if not lines:
        errors.append(f"{path}: empty file (missing version header)")
        return None

    if lines[0].strip() == "---":
        end_idx = None
        for i in range(1, len(lines)):
            if lines[i].strip() == "---":
                end_idx = i
                break
        if end_idx is None:
            errors.append(f"{path}: unterminated frontmatter")
            return None
        for line in lines[1:end_idx]:
            match = FRONTMATTER_VERSION_RE.match(line)
            if match:
                return match.group(1)
        errors.append(f"{path}: missing llman-template-version in frontmatter")
        return None

    match = HTML_VERSION_RE.match(lines[0])
    if not match:
        errors.append(
            f"{path}: missing or invalid llman-template-version header on first line"
        )
        return None
    return match.group(1)


def collect_versions(locale_dir: Path, errors: List[str]) -> Dict[str, str]:
    versions: Dict[str, str] = {}
    for path in sorted(locale_dir.rglob("*.md")):
        rel = path.relative_to(locale_dir).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        version = extract_version(path, lines, errors)
        if version is None:
            continue
        versions[rel] = version
    if not versions:
        errors.append(f"{locale_dir}: no markdown templates found")
    return versions


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    templates_root = repo_root / "templates" / "sdd"
    if not templates_root.exists():
        print("ERROR: templates/sdd not found")
        return 1

    locale_dirs = sorted([p for p in templates_root.iterdir() if p.is_dir()])
    if not locale_dirs:
        print("ERROR: no locale directories found under templates/sdd")
        return 1

    locales = [p.name for p in locale_dirs]
    base_locale = "en" if (templates_root / "en").is_dir() else locales[0]
    base_dir = templates_root / base_locale

    errors: List[str] = []
    base_versions = collect_versions(base_dir, errors)

    for locale_dir in locale_dirs:
        if locale_dir == base_dir:
            continue
        versions = collect_versions(locale_dir, errors)

        base_set = set(base_versions)
        other_set = set(versions)

        for rel in sorted(base_set - other_set):
            errors.append(f"{locale_dir / rel}: missing template (expected {rel})")
        for rel in sorted(other_set - base_set):
            errors.append(f"{locale_dir / rel}: extra template (not in {base_locale})")
        for rel in sorted(base_set & other_set):
            if base_versions[rel] != versions[rel]:
                errors.append(
                    f"{locale_dir / rel}: version {versions[rel]} does not match "
                    f"{base_locale} version {base_versions[rel]}"
                )

    if errors:
        print("SDD template checks failed:")
        for err in errors:
            print(f"- {err}")
        return 1

    locale_list = ", ".join(locales)
    print(f"SDD template checks passed for locales: {locale_list}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
