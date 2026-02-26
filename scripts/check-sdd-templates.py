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
SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def extract_frontmatter_version(
    path: Path, lines: List[str], errors: List[str]
) -> Optional[str]:
    end_idx = None
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            end_idx = i
            break
    if end_idx is None:
        errors.append(f"{path}: unterminated frontmatter")
        return None

    metadata_indent = None
    in_metadata = False
    for line in lines[1:end_idx]:
        if not line.strip():
            continue
        leading = len(line) - len(line.lstrip(" "))
        stripped = line.strip()
        if not line.startswith(" "):
            # top-level key
            if stripped == "metadata:":
                metadata_indent = leading
                in_metadata = True
            else:
                in_metadata = False
            continue
        if not in_metadata or metadata_indent is None:
            continue
        if leading <= metadata_indent:
            in_metadata = False
            continue
        match = FRONTMATTER_VERSION_RE.match(stripped)
        if match:
            return match.group(1)

    errors.append(f"{path}: missing llman-template-version in metadata")
    return None


def extract_version(path: Path, lines: List[str], errors: List[str]) -> Optional[str]:
    if not lines:
        errors.append(f"{path}: empty file (missing version header)")
        return None

    if lines[0].strip() == "---":
        return extract_frontmatter_version(path, lines, errors)

    match = HTML_VERSION_RE.match(lines[0])
    if not match:
        errors.append(
            f"{path}: missing or invalid llman-template-version header on first line"
        )
        return None
    return match.group(1)


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


def collect_versions(locale_dir: Path, errors: List[str]) -> Dict[str, str]:
    versions: Dict[str, str] = {}
    for path in sorted(locale_dir.rglob("*.md")):
        rel = path.relative_to(locale_dir).as_posix()
        lines = path.read_text(encoding="utf-8").splitlines()
        validate_skill_frontmatter(path, lines, errors)
        version = extract_version(path, lines, errors)
        if version is None:
            continue
        versions[rel] = version
    if not versions:
        errors.append(f"{locale_dir}: no markdown templates found")
    return versions


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

    return locales


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    templates_root = repo_root / "templates"
    errors: List[str] = []
    sdd_root = templates_root / "sdd"
    legacy_root = templates_root / "sdd-legacy"

    sdd_locales = validate_markdown_root(sdd_root, errors)

    legacy_locales: List[str] = []
    if legacy_root.exists():
        legacy_locales = validate_markdown_root(legacy_root, errors)

    if errors:
        print("SDD template checks failed:")
        for err in errors:
            print(f"- {err}")
        return 1

    locale_list = ", ".join(sdd_locales)
    if legacy_locales:
        legacy_list = ", ".join(legacy_locales)
        print(
            f"SDD template checks passed for locales: {locale_list} (legacy: {legacy_list})"
        )
    else:
        print(f"SDD template checks passed for locales: {locale_list}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
