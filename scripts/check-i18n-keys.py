#!/usr/bin/env python3
"""i18n key audit for locales/app.yml (rust-i18n).

Two directions:
  1. dead keys   — keys defined in app.yml never referenced in Rust sources
  2. missing keys — `t!("…")` literals in sources that app.yml does not define

Exit code is non-zero when either list is non-empty, so this doubles as a CI
gate. Stdlib-only (no PyYAML) by design: app.yml is a controlled, shallow
nested mapping of `key:` sections whose leaves are `locale: "text"`.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LOCALES = REPO / "locales"
CODE_DIRS = ("src", "crates", "tests")

SECTION_RE = re.compile(r"^(\s*)([A-Za-z0-9_-]+):\s*$")
LEAF_RE = re.compile(r'^(\s*)([A-Za-z0-9_-]+):\s*(?:"(.*)"\s*|\S.*)$')
COMMENT_RE = re.compile(r"^\s*#")
LOCALES_SUPPORTED = {"en", "zh-Hans"}


def parse_locale_tree(path: Path) -> dict:
    """Parse app.yml's nested `key:` mappings into a dict tree.

    Leaves (`en: "text"`) become plain strings; sections become dicts.
    """
    root: dict = {}
    stack: list[tuple[int, dict]] = [(-1, root)]
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw.strip() or COMMENT_RE.match(raw):
            continue
        leaf = LEAF_RE.match(raw)
        section = SECTION_RE.match(raw)
        if not section and not leaf:
            continue
        indent = len((leaf or section).group(1))
        while stack and indent <= stack[-1][0]:
            stack.pop()
        parent = stack[-1][1]
        if section:
            node: dict = {}
            parent[section.group(2)] = node
            stack.append((indent, node))
        else:
            parent[leaf.group(2)] = leaf.group(3) or ""
    return root


def collect_key_paths(node: dict, prefix: str = "") -> set[str]:
    """Dotted rust-i18n keys = every path whose own children are locale texts."""
    keys: set[str] = set()
    for name, value in node.items():
        path = f"{prefix}.{name}" if prefix else name
        if isinstance(value, dict):
            if set(value.keys()) <= LOCALES_SUPPORTED and value:
                keys.add(path)
            else:
                keys |= collect_key_paths(value, path)
    return keys


def collect_used_key_literals() -> set[str]:
    """All `t!("…")` / `t!("…", …)` first-argument literals in Rust sources."""
    used: set[str] = set()
    pattern = re.compile(r'\bt!\(\s*"([^"]+)"')
    for code_dir in CODE_DIRS:
        for file in (REPO / code_dir).rglob("*.rs"):
            for match in pattern.finditer(file.read_text(encoding="utf-8")):
                used.add(match.group(1))
    return used


def key_referenced_in_code(key: str) -> bool:
    for code_dir in CODE_DIRS:
        for file in (REPO / code_dir).rglob("*.rs"):
            if key in file.read_text(encoding="utf-8"):
                return True
    return False


def strip_dead_keys(path: Path, dead: set[str]) -> int:
    """Remove dead leaf sections (`key:` + its locale lines) from app.yml."""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out: list[str] = []
    stack: list[tuple[int, str]] = [(-1, "")]
    skipping = False
    skip_indent = 0
    removed = 0
    for raw in lines:
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            if not skipping:
                out.append(raw)
            continue
        leaf = LEAF_RE.match(raw)
        section = SECTION_RE.match(raw)
        if not leaf and not section:
            out.append(raw)
            continue
        indent = len((leaf or section).group(1))
        key = (leaf or section).group(2)
        while stack and indent <= stack[-1][0]:
            stack.pop()
        prefix = ".".join(name for _, name in stack[1:])
        full = f"{prefix}.{key}" if prefix else key
        if skipping:
            if indent > skip_indent:
                continue
            skipping = False
        if section and full in dead:
            skipping = True
            skip_indent = indent
            removed += 1
            continue
        out.append(raw)
        if section:
            stack.append((indent, key))
    if removed:
        path.write_text("".join(out), encoding="utf-8")
    return removed


def main() -> int:
    if "--fix" in sys.argv:
        defined_before = collect_key_paths(parse_locale_tree(LOCALES / "app.yml"))
        dead = {key for key in defined_before if not key_referenced_in_code(key)}
        removed = strip_dead_keys(LOCALES / "app.yml", dead)
        print(f"removed {removed} dead key blocks from {LOCALES / 'app.yml'}")
        return 0

    defined = collect_key_paths(parse_locale_tree(LOCALES / "app.yml"))
    used = collect_used_key_literals()

    # Keys referenced via dynamic construction (e.g. the sdd.cmdref.* table
    # built from the clap command tree at render time, r139) are invisible to
    # literal scanning — whitelist those namespaces instead of failing.
    DYNAMIC_KEY_PREFIXES = ("sdd.cmdref.",)

    dead = sorted(
        key for key in defined
        if not key_referenced_in_code(key)
        and not key.startswith(DYNAMIC_KEY_PREFIXES)
    )
    missing = sorted(used - defined)

    for key in dead:
        print(f"dead i18n key (defined in app.yml, unused in code): {key}")
    for key in missing:
        print(f"missing i18n key (used in code, not in app.yml): {key}")
    print(f"{len(defined)} defined · {len(used & defined)} used · "
          f"{len(dead)} dead · {len(missing)} missing")

    return 1 if dead or missing else 0


if __name__ == "__main__":
    sys.exit(main())
