#!/usr/bin/env python3
from pathlib import Path
import re
import sys
from typing import Dict, List, Optional

SKILL_NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FORBIDDEN_PROMPT_TOOLING_RE = re.compile(
    r"(?i)/llman-sdd:|\bclaude\b|\bcodex\b|slash commands?"
)
# `llman sdd <subcommand...>` references (flags like --strict terminate the match).
LLMAN_CMD_RE = re.compile(
    r"\bllman\s+sdd\s+[a-z][a-z0-9]*(?:\s+-?[a-z][a-z0-9-]*)*"
)
UNIT_REF_RE = re.compile(r'\{\{\s*unit\("([^"]+)"\)\s*\}\}')
JINJA_BLOCK_RE = re.compile(r"\{%[^%]*%\}")
SPEC_TAG_RE = re.compile(r"@(?:human|executable|manual)\b|@req:[A-Za-z0-9_-]+")
HEADING_RE = re.compile(r"^(#{1,6}) ")
# Files whose body is intentionally identical across locales carry this marker.
LOCALE_INDEPENDENT_MARKER = "sdd-template: locale-independent"


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


def strip_code_fences(text: str) -> str:
    out: List[str] = []
    in_fence = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append(line)
    return "\n".join(out)


def locale_body_markers(text: str) -> Dict[str, object]:
    prose = strip_code_fences(text)
    return {
        "llman_cmds": sorted(set(LLMAN_CMD_RE.findall(prose))),
        "unit_refs": UNIT_REF_RE.findall(text),
        "jinja_blocks": JINJA_BLOCK_RE.findall(text),
        "spec_tags": sorted(SPEC_TAG_RE.findall(text)),
        "heading_levels": [len(m.group(1)) for m in HEADING_RE.finditer(prose)],
    }


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


def validate_locale_body_parity(
    base_dir: Path,
    base_locale: str,
    other_dir: Path,
    rel_files: List[str],
    errors: List[str],
) -> None:
    """Compare per-file body markers between the base locale and another locale.

    Catches the drift classes that filename parity cannot see: untranslated
    bodies, diverging CLI command references, diverging unit/jinja structure,
    diverging spec tags, and diverging heading outlines.
    """
    for rel in rel_files:
        base_path = base_dir / rel
        other_path = other_dir / rel
        base_text = base_path.read_text(encoding="utf-8")
        other_text = other_path.read_text(encoding="utf-8")

        if LOCALE_INDEPENDENT_MARKER in base_text and LOCALE_INDEPENDENT_MARKER in other_text:
            continue

        if base_text == other_text:
            errors.append(
                f"{other_path}: body is byte-identical to {base_locale} "
                f"(untranslated? add '{LOCALE_INDEPENDENT_MARKER}' if intentionally shared)"
            )

        base_markers = locale_body_markers(base_text)
        other_markers = locale_body_markers(other_text)
        label = f"{other_path}"
        if base_markers["llman_cmds"] != other_markers["llman_cmds"]:
            only_base = sorted(set(base_markers["llman_cmds"]) - set(other_markers["llman_cmds"]))
            only_other = sorted(set(other_markers["llman_cmds"]) - set(base_markers["llman_cmds"]))
            errors.append(
                f"{label}: llman CLI command references diverge "
                f"(only {base_locale}: {only_base}; only this locale: {only_other})"
            )
        if base_markers["unit_refs"] != other_markers["unit_refs"]:
            errors.append(
                f"{label}: unit references diverge "
                f"({base_locale}: {base_markers['unit_refs']}; this locale: {other_markers['unit_refs']})"
            )
        if base_markers["jinja_blocks"] != other_markers["jinja_blocks"]:
            errors.append(
                f"{label}: jinja blocks diverge "
                f"({base_locale}: {base_markers['jinja_blocks']}; this locale: {other_markers['jinja_blocks']})"
            )
        if base_markers["spec_tags"] != other_markers["spec_tags"]:
            errors.append(
                f"{label}: spec tags diverge "
                f"({base_locale}: {base_markers['spec_tags']}; this locale: {other_markers['spec_tags']})"
            )
        if base_markers["heading_levels"] != other_markers["heading_levels"]:
            errors.append(
                f"{label}: heading outline diverges "
                f"({base_locale}: {base_markers['heading_levels']}; this locale: {other_markers['heading_levels']})"
            )


def validate_markdown_root(templates_root: Path, errors: List[str]) -> List[str]:
    if not templates_root.exists():
        errors.append(f"ERROR: {templates_root} not found")
        return []

    # Locale roots are exactly the dirs that carry a `skills/` tree; other
    # first-level dirs (e.g. `shared/` locale-agnostic assets) are skipped.
    locale_dirs = sorted(
        [
            p
            for p in templates_root.iterdir()
            if p.is_dir() and (p / "skills").is_dir()
        ]
    )
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

        common = sorted(base_set & other_set)
        validate_locale_body_parity(base_dir, base_locale, locale_dir, common, errors)

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

    # Shared (locale-agnostic) assets: light sanity checks only — existence and
    # a closing html tag. Deep checks would duplicate the runtime renderer.
    shared_dir = sdd_root / "shared"
    if shared_dir.is_dir():
        for path in sorted(shared_dir.glob("*.html")):
            text = path.read_text(encoding="utf-8")
            if "</html>" not in text:
                errors.append(f"{path}: missing closing </html>")
    else:
        errors.append(f"{shared_dir}: missing shared templates directory")

    if errors:
        print("SDD template checks failed:")
        for err in errors:
            print(f"- {err}")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
