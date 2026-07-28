"""Skill-gate hard assertion for sdd_skill_gate_v1.

Simplified version of sdd_llmanspec_styles_v1/assertions/sdd_gate.py:
- Drops spec-style fence checks (skill-gate measures skill template quality,
  not spec storage format).
- Variant dimension is baseline/candidate (from provider id), not ison/toon/yaml.
- The hard gate remains `llman sdd validate --all --strict --no-interactive`.
"""
from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any, Dict, Optional


class GateContext:
    # NOTE: Avoid `@dataclass`; see sdd_llmanspec_styles_v1/assertions/sdd_gate.py
    # for the promptfoo importlib compat rationale.
    def __init__(self, provider: str, workspace_dir: Path, config_dir: Path, variant: str) -> None:
        self.provider = provider
        self.workspace_dir = workspace_dir
        self.config_dir = config_dir
        self.variant = variant


def _provider_id(context: Dict[str, Any]) -> str:
    provider = context.get("provider")
    if isinstance(provider, dict):
        provider_id = provider.get("id")
        if isinstance(provider_id, str) and provider_id.strip():
            return provider_id.strip()
    if isinstance(provider, str) and provider.strip():
        return provider.strip()
    return "unknown-provider"


def _select_variant(provider_id: str) -> str:
    lowered = provider_id.lower()
    if "baseline" in lowered:
        return "baseline"
    if "candidate" in lowered:
        return "candidate"
    return "unknown"


def _tool_calls(context: Dict[str, Any]) -> list[dict[str, Any]]:
    provider_resp = context.get("providerResponse")
    if not isinstance(provider_resp, dict):
        return []
    meta = provider_resp.get("metadata")
    if not isinstance(meta, dict):
        return []
    tool_calls = meta.get("toolCalls")
    if tool_calls is None:
        tool_calls = meta.get("tool_calls")
    if isinstance(tool_calls, list):
        out: list[dict[str, Any]] = []
        for item in tool_calls:
            if isinstance(item, dict):
                out.append(item)
        return out
    return []


def _find_workspace_root(path: Path) -> Optional[Path]:
    cur = path
    for _ in range(6):
        if (cur / "llmanspec" / "config.yaml").exists():
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    return None


def _maybe_workspace_from_pwd_toolcall(tool_call: dict[str, Any]) -> Optional[Path]:
    name = tool_call.get("name")
    if not isinstance(name, str) or name.lower() != "bash":
        return None
    tc_input = tool_call.get("input")
    if not isinstance(tc_input, dict):
        return None
    command = tc_input.get("command")
    if not isinstance(command, str) or "pwd" not in command.lower():
        return None
    output = tool_call.get("output")
    if output is None:
        output = tool_call.get("result")
    if not isinstance(output, str) or not output.strip():
        return None
    first_line = output.splitlines()[0].strip()
    if not first_line.startswith("/"):
        return None
    candidate = Path(first_line)
    if not candidate.is_dir():
        return None
    return _find_workspace_root(candidate)


def _maybe_workspace_from_config_read_toolcall(tool_call: dict[str, Any]) -> Optional[Path]:
    name = tool_call.get("name")
    if not isinstance(name, str) or name.lower() != "read":
        return None
    tc_input = tool_call.get("input")
    if not isinstance(tc_input, dict):
        return None
    file_path = tc_input.get("file_path")
    if file_path is None:
        file_path = tc_input.get("filePath")
    if not isinstance(file_path, str) or not file_path.strip():
        return None
    path = Path(file_path)
    if path.name != "config.yaml":
        return None
    if path.parent.name != "llmanspec":
        return None
    workspace_dir = path.parent.parent
    if workspace_dir.is_dir():
        return _find_workspace_root(workspace_dir) or workspace_dir
    return None


def _infer_workspace_dir(context: Dict[str, Any]) -> Optional[Path]:
    for tc in _tool_calls(context):
        ws = _maybe_workspace_from_config_read_toolcall(tc)
        if ws is not None:
            return ws
    for tc in _tool_calls(context):
        ws = _maybe_workspace_from_pwd_toolcall(tc)
        if ws is not None:
            return ws
    return None


def _infer_config_dir(workspace_dir: Path, variant: str) -> Optional[Path]:
    # Prefer runner-exported env vars (SDD_CONFIGDIR_BASELINE / SDD_CONFIGDIR_CANDIDATE).
    variant_upper = variant.upper()
    env_value = os.environ.get(f"SDD_CONFIGDIR_{variant_upper}")
    if env_value:
        candidate = Path(env_value)
        if candidate.is_dir():
            return candidate
    # Fallback to runner layout: <work_dir>/configs/<variant>
    if workspace_dir.parent.name == "workspaces":
        work_dir = workspace_dir.parent.parent
        candidate = work_dir / "configs" / variant
        if candidate.is_dir():
            return candidate
    return None


def _workspace_paths_for(variant: str) -> tuple[Optional[str], Optional[str]]:
    variant_upper = variant.upper()
    return (
        os.environ.get(f"SDD_WORKDIR_{variant_upper}"),
        os.environ.get(f"SDD_CONFIGDIR_{variant_upper}"),
    )


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
    variant = _select_variant(provider_id)

    workspace_dir = _infer_workspace_dir(context)
    if workspace_dir is None:
        # Fallback: map via provider id + runner-exported env vars.
        workdir_str, configdir_str = _workspace_paths_for(variant)
        if workdir_str and configdir_str:
            return GateContext(
                provider=provider_id,
                workspace_dir=Path(workdir_str),
                config_dir=Path(configdir_str),
                variant=variant,
            )
        raise RuntimeError(
            "Unable to infer workspace_dir from assertion context. "
            "Expected providerResponse.metadata.toolCalls to include `pwd` output or a config.yaml read. "
            f"provider_id={provider_id} variant={variant}"
        )

    env_workdir = os.environ.get(f"SDD_WORKDIR_{variant.upper()}")
    if env_workdir:
        expected_ws = Path(env_workdir)
        if not expected_ws.is_dir():
            raise RuntimeError(
                "Runner exported SDD_WORKDIR for this variant is not a directory. "
                f"variant={variant} path={expected_ws} provider_id={provider_id}"
            )
        if workspace_dir.resolve() != expected_ws.resolve():
            raise RuntimeError(
                "Workspace dir mismatch for hard gate. "
                f"variant={variant} expected={expected_ws} actual={workspace_dir} provider_id={provider_id}"
            )

    config_dir = _infer_config_dir(workspace_dir, variant)
    if config_dir is None:
        raise RuntimeError(
            "Unable to infer config_dir for hard gate. "
            f"Expected SDD_CONFIGDIR_{variant.upper()} or runner layout <work_dir>/configs/{variant}. "
            f"workspace_dir={workspace_dir} provider_id={provider_id}"
        )

    return GateContext(
        provider=provider_id,
        workspace_dir=workspace_dir,
        config_dir=config_dir,
        variant=variant,
    )


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
        validate_issue = _hard_validate(gate)
        if validate_issue:
            return {"pass": False, "score": 0, "reason": validate_issue}
        return {
            "pass": True,
            "score": 1,
            "reason": f"OK ({gate.variant})",
        }
    except Exception as e:  # noqa: BLE001
        return {"pass": False, "score": 0, "reason": f"Assertion error: {e}"}
