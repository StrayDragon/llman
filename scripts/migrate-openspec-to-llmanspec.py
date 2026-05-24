#!/usr/bin/env python3
"""Migrate OpenSpec markdown specs to llmanspec toon format.

Parses OpenSpec-style markdown specs (# <name> Specification / ## Purpose /
### Requirement: / #### Scenario:) and converts them to llmanspec toon specs
via the `llman sdd spec` CLI helpers.

Usage:
    python migrate-openspec-to-llmanspec.py [OPTIONS]

Options:
    --source DIR    Source OpenSpec specs directory (default: openspec/specs)
    --target DIR    Target llmanspec specs directory (default: llmanspec/specs)
    --root DIR      Project root (default: auto-detect from CWD or script location)
    --dry-run       Parse and report without writing files
    --force         Overwrite existing specs in target
    --scope GLOB    Only migrate specs matching glob pattern (e.g. 'yaml-dsl-*')
    --frontmatter   Custom YAML frontmatter values as KEY=VALUE pairs
                    (e.g. --frontmatter valid_scope=src/ valid_commands='pytest')

Requirements:
    - `llman` CLI must be installed and available in PATH
    - Target llmanspec project must be initialized (`llman sdd init`)

Spec Format (input):
    # <name> Specification
    ## Purpose
    <purpose text>
    ## Requirements
    ### Requirement: <title>
    <statement text with MUST/SHALL keywords>
    #### Scenario: <scenario name>
    - **GIVEN** <precondition>
    - **WHEN** <trigger>
    - **THEN** <expected outcome>

Spec Format (output — toon):
    ---
    llman_spec_valid_scope: [<scope>]
    llman_spec_valid_commands: [<commands>]
    llman_spec_evidence: [<evidence>]
    ---
    ```toon
    kind: llman.sdd.spec
    name: "<name>"
    purpose: "<one-line purpose>"
    requirements[N]{req_id,title,statement}:
      r1,<title>,"<statement>"
    scenarios[N]{req_id,id,given,when,then}:
      r1,<scenario-id>,"<given>","<when>","<then>"
    ```

Known Limitations:
    - Statements starting with '- ' may confuse clap CLI argument parsing;
      such requirements are skipped and reported for manual fix.
    - Very long statements (>500 chars) are truncated for toon compatibility.
    - Scenario IDs are slugified from scenario names; duplicates get a suffix.
"""

from __future__ import annotations

import argparse
import fnmatch
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Tuple


@dataclass
class Scenario:
    id: str
    given: str
    when: str
    then: str


@dataclass
class Requirement:
    title: str
    statement: str
    scenarios: List[Scenario] = field(default_factory=list)


@dataclass
class ParsedSpec:
    name: str
    purpose: str
    requirements: List[Requirement] = field(default_factory=list)
    raw_text: str = ""


@dataclass
class MigrationResult:
    name: str
    status: str  # ok, partial, error, skip
    req_count: int = 0
    scenario_count: int = 0
    errors: List[str] = field(default_factory=list)
    reason: str = ""


def run_llman(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    cmd = ["llman", "sdd", *args]
    return subprocess.run(cmd, cwd=str(root), capture_output=True, text=True, check=check)


def parse_openspec_md(path: Path) -> ParsedSpec:
    """Parse an OpenSpec markdown spec into structured data.

    Expected format:
        # <name> Specification
        ## Purpose
        <text>
        ## Requirements
        ### Requirement: <title>
        <statement>
        #### Scenario: <name>
        - **GIVEN/WHEN/THEN** <text>
    """
    text = path.read_text(encoding="utf-8")
    spec = ParsedSpec(name="", purpose="", raw_text=text)

    name_m = re.match(r"^#\s+(\S+)\s+Specification", text)
    if name_m:
        spec.name = name_m.group(1)

    purpose_m = re.search(r"##\s+Purpose\s*\n(.*?)(?=\n##\s|\Z)", text, re.DOTALL)
    if purpose_m:
        purpose_text = purpose_m.group(1).strip()
        purpose_text = re.sub(r"\*\*.*?\*\*\s*", "", purpose_text)
        purpose_lines = [line.strip() for line in purpose_text.split("\n") if line.strip()]
        spec.purpose = " ".join(purpose_lines)[:300] if purpose_lines else "TBD"

    req_pattern = re.compile(
        r"###\s+Requirement:\s*(.*?)\n(.*?)(?=\n###\s+Requirement:|\n##\s|\Z)",
        re.DOTALL,
    )

    for req_match in req_pattern.finditer(text):
        req_title = req_match.group(1).strip()
        req_body = req_match.group(2).strip()

        body_before_scenarios = re.split(r"\n####\s+Scenario:", req_body)[0].strip()
        stmt_lines = []
        for line in body_before_scenarios.split("\n"):
            line = line.strip()
            if not line:
                continue
            if line.startswith("## ") or line.startswith("### "):
                break
            stmt_lines.append(line)
        statement = " ".join(stmt_lines) if stmt_lines else req_title

        scenario_pattern = re.compile(
            r"####\s+Scenario:\s*(.*?)\n(.*?)(?=\n####\s+Scenario:|\n###\s+Requirement:|\n##\s|\Z)",
            re.DOTALL,
        )

        scenarios = []
        seen_ids: set[str] = set()
        for sc_match in scenario_pattern.finditer(req_body):
            sc_name = sc_match.group(1).strip()
            sc_body = sc_match.group(2).strip()
            given, when, then = _extract_gherkin(sc_body)
            sc_id = _slugify(sc_name)
            if sc_id in seen_ids:
                sc_id = f"{sc_id}-{len(seen_ids)}"
            seen_ids.add(sc_id)
            scenarios.append(Scenario(id=sc_id, given=given, when=when, then=then))

        spec.requirements.append(Requirement(title=req_title, statement=statement, scenarios=scenarios))

    return spec


def _extract_gherkin(body: str) -> Tuple[str, str, str]:
    """Extract GIVEN/WHEN/THEN from a scenario body."""
    given = ""
    when = ""
    then = ""

    given_m = re.search(r"\*\*(?:GIVEN|Given)\*\*\s*(.*?)(?=\n\s*-\s*\*\*|\Z)", body, re.DOTALL)
    when_m = re.search(r"\*\*(?:WHEN|When)\*\*\s*(.*?)(?=\n\s*-\s*\*\*|\Z)", body, re.DOTALL)
    then_m = re.search(r"\*\*(?:THEN|Then)\*\*\s*(.*?)(?=\n\s*-\s*\*\*|\Z)", body, re.DOTALL)

    if given_m:
        given = _clean_gherkin(given_m.group(1))
    if when_m:
        when = _clean_gherkin(when_m.group(1))
    if then_m:
        then = _clean_gherkin(then_m.group(1))

    if not when and not then:
        lines = [line.strip() for line in body.split("\n") if line.strip()]
        combined = " ".join(lines)
        when = combined[:200] if combined else "condition is met"
        then = "expected behavior occurs"

    if not when:
        when = "condition is met"
    if not then:
        then = "expected behavior occurs"

    return given, when, then


def _clean_gherkin(text: str) -> str:
    text = text.strip()
    lines = []
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("- **"):
            break
        line = re.sub(r"^\s*-\s*", "", line)
        line = re.sub(r"\*\*(AND|and|And)\*\*\s*", "", line)
        if line:
            lines.append(line)
    result = " ".join(lines)
    return re.sub(r"\s+", " ", result).strip()


def _slugify(name: str, max_len: int = 60) -> str:
    """Convert a scenario name to a kebab-case id."""
    slug = name.lower()
    slug = re.sub(r"[^a-z0-9\u4e00-\u9fff]+", "-", slug)
    slug = slug.strip("-")
    return slug[:max_len] if slug else "default"


def _toon_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def migrate_spec(
    spec: ParsedSpec,
    root: Path,
    target_dir: Path,
    *,
    dry_run: bool = False,
    force: bool = False,
    frontmatter: Optional[dict] = None,
) -> MigrationResult:
    """Migrate a single parsed spec to llmanspec toon format."""
    if target_dir.exists() and not force:
        return MigrationResult(name=spec.name, status="skip", reason="already exists")

    if dry_run:
        return MigrationResult(
            name=spec.name,
            status="ok",
            req_count=len(spec.requirements),
            scenario_count=sum(len(r.scenarios) for r in spec.requirements),
        )

    skeleton_args = ["spec", "skeleton", spec.name]
    if force:
        skeleton_args.append("--force")
    r = run_llman(root, *skeleton_args, check=False)
    if r.returncode != 0:
        return MigrationResult(name=spec.name, status="error", reason=f"skeleton failed: {r.stderr.strip()}")

    errors: List[str] = []
    total_scenarios = 0

    for i, req in enumerate(spec.requirements, 1):
        req_id = f"r{i}"
        title = req.title[:80]
        statement = req.statement[:500]

        if not re.search(r"\b(MUST|SHALL|SHOULD|MUST NOT|SHALL NOT)\b", statement):
            statement = f"System MUST {statement}"

        r = run_llman(
            root,
            "spec", "add-requirement", spec.name, req_id,
            "--title", title,
            "--statement", statement,
            check=False,
        )
        if r.returncode != 0:
            errors.append(f"req {req_id} ({title[:30]}...): {r.stderr.strip()}")
            continue

        for j, sc in enumerate(req.scenarios, 1):
            sc_id = sc.id or f"s{j}"
            args = [
                "spec", "add-scenario", spec.name, req_id, sc_id,
                "--when", sc.when,
                "--then", sc.then,
            ]
            if sc.given:
                args.extend(["--given", sc.given])
            r = run_llman(root, *args, check=False)
            if r.returncode != 0:
                errors.append(f"scenario {req_id}/{sc_id}: {r.stderr.strip()}")
            else:
                total_scenarios += 1

    _update_purpose(target_dir / "spec.md", spec.purpose)

    if frontmatter:
        _update_frontmatter(target_dir / "spec.md", spec.name, frontmatter)

    status = "ok" if not errors else "partial"
    return MigrationResult(
        name=spec.name,
        status=status,
        req_count=len(spec.requirements),
        scenario_count=total_scenarios,
        errors=errors,
    )


def _update_purpose(spec_file: Path, purpose: str):
    if not spec_file.exists() or not purpose:
        return
    content = spec_file.read_text(encoding="utf-8")
    escaped = _toon_escape(purpose[:200]) if purpose != "TBD" else "TBD"
    content = content.replace(
        'purpose: "TODO: Describe this capability and its purpose."',
        f'purpose: "{escaped}"',
    )
    spec_file.write_text(content, encoding="utf-8")


def _update_frontmatter(spec_file: Path, spec_name: str, fm: dict):
    if not spec_file.exists():
        return
    content = spec_file.read_text(encoding="utf-8")
    if "valid_scope" in fm:
        content = content.replace("  - src/\n  - tests/", f"  - {fm['valid_scope']}")
    if "valid_commands" in fm:
        content = content.replace("  - cargo test", f"  - {fm['valid_commands']}")
    else:
        content = content.replace(
            "  - cargo test",
            f"  - llman sdd validate {spec_name} --type spec --strict --no-interactive",
        )
    if "evidence" in fm:
        content = content.replace(
            '  - "TODO: add evidence (CI link, benchmark output, etc.)"',
            f"  - {fm['evidence']}",
        )
    else:
        content = content.replace(
            '  - "TODO: add evidence (CI link, benchmark output, etc.)"',
            "  - migrated from openspec",
        )
    spec_file.write_text(content, encoding="utf-8")


def find_project_root(start: Path) -> Optional[Path]:
    """Walk up from start to find a directory containing llmanspec/."""
    for parent in [start, *start.parents]:
        if (parent / "llmanspec").is_dir():
            return parent
    return None


def main():
    parser = argparse.ArgumentParser(
        description="Migrate OpenSpec markdown specs to llmanspec toon format.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--source", type=Path, help="Source OpenSpec specs directory (default: openspec/specs)")
    parser.add_argument("--target", type=Path, help="Target llmanspec specs directory (default: llmanspec/specs)")
    parser.add_argument("--root", type=Path, help="Project root directory")
    parser.add_argument("--dry-run", action="store_true", help="Parse and report without writing files")
    parser.add_argument("--force", action="store_true", help="Overwrite existing specs")
    parser.add_argument("--scope", help="Glob pattern to filter spec names (e.g. 'yaml-dsl-*')")
    parser.add_argument("--frontmatter", nargs="*", metavar="KEY=VALUE", help="Custom frontmatter values")
    args = parser.parse_args()

    root = args.root
    if not root:
        root = find_project_root(Path.cwd())
        if not root:
            root = find_project_root(Path(__file__).resolve().parent)
    if not root:
        print("ERROR: Cannot find project root (no llmanspec/ directory found).", file=sys.stderr)
        print("Use --root to specify the project root.", file=sys.stderr)
        sys.exit(1)

    source = args.source or root / "openspec" / "specs"
    target = args.target or root / "llmanspec" / "specs"

    if not source.exists():
        print(f"ERROR: Source directory does not exist: {source}", file=sys.stderr)
        sys.exit(1)

    if not target.parent.exists():
        print(f"ERROR: Target parent directory does not exist: {target.parent}", file=sys.stderr)
        sys.exit(1)

    fm = {}
    if args.frontmatter:
        for kv in args.frontmatter:
            if "=" not in kv:
                print(f"ERROR: Invalid --frontmatter value: {kv} (expected KEY=VALUE)", file=sys.stderr)
                sys.exit(1)
            k, v = kv.split("=", 1)
            fm[k] = v

    r = subprocess.run(["llman", "--version"], capture_output=True, text=True, check=False)
    if r.returncode != 0:
        print("ERROR: llman CLI not found. Install it first.", file=sys.stderr)
        sys.exit(1)
    print(f"Using llman {r.stdout.strip()}")
    print(f"Source: {source}")
    print(f"Target: {target}")
    if args.dry_run:
        print("Mode: DRY RUN (no files will be written)")
    print()

    spec_dirs = sorted(d for d in source.iterdir() if d.is_dir() and (d / "spec.md").exists())

    if args.scope:
        spec_dirs = [d for d in spec_dirs if fnmatch.fnmatch(d.name, args.scope)]

    print(f"Found {len(spec_dirs)} specs to migrate\n")

    results: dict[str, list[MigrationResult]] = {"ok": [], "partial": [], "error": [], "skip": []}

    for i, spec_dir in enumerate(spec_dirs, 1):
        print(f"[{i:3d}/{len(spec_dirs)}] {spec_dir.name}...", end=" ", flush=True)

        parsed = parse_openspec_md(spec_dir / "spec.md")
        if not parsed.name:
            parsed.name = spec_dir.name

        result = migrate_spec(
            parsed,
            root,
            target / spec_dir.name,
            dry_run=args.dry_run,
            force=args.force,
            frontmatter=fm if fm else None,
        )
        print(f"{result.status} (reqs={result.req_count}, scenarios={result.scenario_count})")
        results[result.status].append(result)

        if result.errors:
            for err in result.errors:
                print(f"         ⚠ {err}")

    print(f"\n{'=' * 60}")
    print("Migration summary:")
    print(f"  OK:      {len(results['ok'])}")
    print(f"  Partial: {len(results['partial'])}")
    print(f"  Error:   {len(results['error'])}")
    print(f"  Skip:    {len(results['skip'])}")

    total_reqs = sum(r.req_count for rs in results.values() for r in rs)
    total_scenarios = sum(r.scenario_count for rs in results.values() for r in rs)
    print(f"\n  Total requirements: {total_reqs}")
    print(f"  Total scenarios:    {total_scenarios}")

    if results["error"]:
        print("\nFailed specs:")
        for r in results["error"]:
            print(f"  - {r.name}: {r.reason}")

    if results["partial"]:
        print("\nPartially migrated specs (some requirements/scenarios failed):")
        for r in results["partial"]:
            print(f"  - {r.name}: {len(r.errors)} error(s)")
            for err in r.errors:
                print(f"      {err}")

    if not args.dry_run and (results["ok"] or results["partial"]):
        print("\nNext steps:")
        print("  1. Run: llman sdd validate --all --strict --no-interactive")
        print("  2. Fix any validation errors in partial specs")
        print("  3. Review and commit the migrated specs")


if __name__ == "__main__":
    main()
