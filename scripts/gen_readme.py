#!/usr/bin/env python3
"""Regenerate the auto-managed sections of README.md.

Managed regions are delimited in README.md by marker pairs:

    <!-- README:GENERATED <name> START -->
    ...generated content...
    <!-- README:GENERATED END -->

Sections and their inputs:
- version  <- `[workspace.package] version` in Cargo.toml
- commands <- `--help` output of the `llman` binary (top level + sdd/x/tool)

Anything outside the markers is hand-written and never touched.

Usage:
  scripts/gen_readme.py           # regenerate README.md in place
  scripts/gen_readme.py --check   # exit 1 when README.md is stale (no write)

Run `just readme` after changing the CLI surface or bumping the workspace
version; `just check-readme` (wired into `just check-all`) fails CI on drift.
"""

from __future__ import annotations

import difflib
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README_PATH = ROOT / "README.md"

MARKER_RE = re.compile(
    r"(<!-- README:GENERATED (?P<name>\w+) START -->\n)"
    r"(?P<body>.*?)"
    r"(<!-- README:GENERATED END -->)",
    re.DOTALL,
)

# CLI groups rendered as their own sub-tables (top level is always rendered).
COMMAND_GROUPS = [
    ("llman sdd", ["sdd"]),
    ("llman x", ["x"]),
    ("llman tool", ["tool"]),
]


def workspace_version() -> str:
    with open(ROOT / "Cargo.toml", "rb") as f:
        data = tomllib.load(f)
    return data["workspace"]["package"]["version"]


def run_help(args: list[str]) -> str:
    # `--help` exits inside clap parsing, before any config-dir logic runs,
    # so this is safe to execute inside the llman dev project itself.
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--bin", "llman", "--", *args, "--help"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(f"`llman {' '.join(args)} --help` failed:\n{proc.stderr}")
    return proc.stdout


def parse_commands(help_text: str) -> list[tuple[str, str]]:
    """Extract the `Commands:` section as (name, description) pairs.

    clap wraps long descriptions onto continuation lines indented deeper
    than the entry column; those lines are folded back into the entry.
    """
    lines = help_text.splitlines()
    start = next(
        (i for i, line in enumerate(lines) if line.strip() == "Commands:"),
        None,
    )
    if start is None:
        raise SystemExit("no 'Commands:' section found in help output")

    entries: list[list[str]] = []
    for line in lines[start + 1 :]:
        if not line.strip():
            break
        entry = re.match(r"^  (\S+)\s{2,}(.+)$", line)
        if entry:
            name, desc = entry.group(1), entry.group(2).strip()
            if name == "help":  # clap built-in, not part of the tool surface
                continue
            entries.append([name, desc])
        else:
            cont = re.match(r"^\s{6,}(\S.*)$", line)
            if cont and entries:
                entries[-1][1] += " " + cont.group(1).strip()
    if not entries:
        raise SystemExit("no commands parsed from help output")
    return [(name, desc) for name, desc in entries]


def render_table(rows: list[tuple[str, str]], label: str) -> list[str]:
    out = [f"| {label} | 说明 |", "|---|---|"]
    out += [f"| `{name}` | {desc} |" for name, desc in rows]
    return out


def gen_version(version: str) -> str:
    return "\n".join(
        [
            "```bash",
            "# crates.io（llman-core / llman-sdd / gherkin-zh 依赖一并安装）",
            f"cargo install llman --version {version}",
            "```",
        ]
    )


def gen_commands() -> str:
    out = ["**`llman`** 顶层命令组：", ""]
    out += render_table(parse_commands(run_help([])), "命令")

    for title, args in COMMAND_GROUPS:
        out += ["", f"**`{title}`**：", ""]
        out += render_table(parse_commands(run_help(args)), "子命令")

    out += [
        "",
        "> 独立二进制 `llmanspec` ≡ `llman sdd`（参数一致，供不带主 CLI 的场景使用）；`llman self` 提供 schema 生成与 shell 补全。",
    ]
    return "\n".join(out)


def generate(name: str) -> str:
    if name == "version":
        return gen_version(workspace_version())
    if name == "commands":
        return gen_commands()
    raise SystemExit(f"unknown generated section: {name}")


def regenerate(readme: str) -> str:
    names = [m.group("name") for m in MARKER_RE.finditer(readme)]
    expected = ["version", "commands"]
    if sorted(names) != sorted(expected):
        raise SystemExit(
            f"README markers mismatch: expected {expected}, found {names}"
        )

    def repl(match: re.Match) -> str:
        name = match.group("name")
        return f"{match.group(1)}{generate(name)}\n{match.group(4)}"

    return MARKER_RE.sub(repl, readme)


def main() -> None:
    check = "--check" in sys.argv[1:]
    readme = README_PATH.read_text(encoding="utf-8")
    updated = regenerate(readme)
    if updated == readme:
        print("README.md up to date")
        return
    if check:
        diff = difflib.unified_diff(
            readme.splitlines(),
            updated.splitlines(),
            fromfile="README.md",
            tofile="README.md (generated)",
            lineterm="",
        )
        raise SystemExit(
            "README.md is stale — run `just readme`:\n"
            + "\n".join(diff)
        )
    README_PATH.write_text(updated, encoding="utf-8")
    print("README.md regenerated")


if __name__ == "__main__":
    main()
