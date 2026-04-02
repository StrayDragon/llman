from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional


@dataclass(frozen=True)
class GateContext:
    provider: str
    workspace_dir: Path
    config_dir: Path
    expected_style: str


def _provider_id(context: Dict[str, Any]) -> str:
    provider = context.get("provider")
    if isinstance(provider, str) and provider.strip():
        return provider.strip()
    return "unknown-provider"


def _select_style(provider_id: str) -> str:
    lowered = provider_id.lower()
    if "ison" in lowered:
        return "ison"
    if "toon" in lowered:
        return "toon"
    if "yaml" in lowered:
        return "yaml"
    return "unknown"


def _workspace_paths_for(style: str) -> tuple[Optional[str], Optional[str]]:
    style_upper = style.upper()
    return (
        os.environ.get(f"SDD_WORKDIR_{style_upper}"),
        os.environ.get(f"SDD_CONFIGDIR_{style_upper}"),
    )


def _read_text(path: Path, max_bytes: int = 200_000) -> str:
    data = path.read_bytes()
    if len(data) > max_bytes:
        data = data[:max_bytes]
    return data.decode("utf-8", errors="replace")


def _run(cmd: list[str], cwd: Path, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd),
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def _build_gate_context(context: Dict[str, Any]) -> GateContext:
    provider_id = _provider_id(context)
    expected_style = _select_style(provider_id)
    workdir_str, configdir_str = _workspace_paths_for(expected_style)
    if not workdir_str or not configdir_str:
        raise RuntimeError(
            "Missing runner env vars for assertion. "
            f"Expected SDD_WORKDIR_{expected_style.upper()} and SDD_CONFIGDIR_{expected_style.upper()}."
        )
    return GateContext(
        provider=provider_id,
        workspace_dir=Path(workdir_str),
        config_dir=Path(configdir_str),
        expected_style=expected_style,
    )


def _check_style_fences(gate: GateContext) -> Optional[str]:
    config_path = gate.workspace_dir / "llmanspec" / "config.yaml"
    if not config_path.exists():
        return f"Missing config: {config_path}"
    config_text = _read_text(config_path)
    if f"spec_style: {gate.expected_style}" not in config_text:
        return (
            f"Spec style mismatch. Expected spec_style: {gate.expected_style}. "
            f"Found config:\n{config_text}"
        )

    cap = "eval-style-cap"
    main_spec = gate.workspace_dir / "llmanspec" / "specs" / cap / "spec.md"
    if not main_spec.exists():
        return f"Missing main spec: {main_spec}"
    main_text = _read_text(main_spec)
    fence = f"```{gate.expected_style}"
    if fence not in main_text:
        return f"Main spec missing code fence {fence}: {main_spec}"

    delta_spec = (
        gate.workspace_dir
        / "llmanspec"
        / "changes"
        / "eval-style-change"
        / "specs"
        / cap
        / "spec.md"
    )
    if not delta_spec.exists():
        return f"Missing delta spec: {delta_spec}"
    delta_text = _read_text(delta_spec)
    if fence not in delta_text:
        return f"Delta spec missing code fence {fence}: {delta_spec}"

    return None


def _hard_validate(gate: GateContext) -> Optional[str]:
    llman_bin = gate.workspace_dir / ".llman-bin" / "llman"
    if not llman_bin.exists():
        return f"Missing llman binary in workspace: {llman_bin}"

    env = dict(os.environ)
    env["LLMAN_CONFIG_DIR"] = str(gate.config_dir)
    proc = _run(
        [str(llman_bin), "sdd", "validate", "--all", "--strict", "--no-interactive"],
        cwd=gate.workspace_dir,
        env=env,
    )
    if proc.returncode != 0:
        return (
            "Hard gate failed: llman sdd validate --all --strict --no-interactive\n"
            f"exit={proc.returncode}\n"
            f"stdout:\n{proc.stdout}\n"
            f"stderr:\n{proc.stderr}\n"
        )
    return None


def get_assert(output: Any, context: Dict[str, Any]) -> Dict[str, Any]:
    try:
        gate = _build_gate_context(context)
        style_issue = _check_style_fences(gate)
        if style_issue:
            return {"pass": False, "score": 0, "reason": style_issue}

        validate_issue = _hard_validate(gate)
        if validate_issue:
            return {"pass": False, "score": 0, "reason": validate_issue}

        return {
            "pass": True,
            "score": 1,
            "reason": f"OK ({gate.expected_style})",
        }
    except Exception as e:  # noqa: BLE001
        return {"pass": False, "score": 0, "reason": f"Assertion error: {e}"}
