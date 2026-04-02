from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path
from typing import Any, Dict, Optional


class GateContext:
    # NOTE: Avoid `@dataclass` here. Promptfoo loads Python assertions via a custom importlib flow
    # (see `promptfoo/dist/src/python/wrapper.py`), and on some Python versions the dataclasses
    # decorator expects the module to be registered in `sys.modules`, which isn't guaranteed.
    def __init__(self, provider: str, workspace_dir: Path, config_dir: Path, expected_style: str) -> None:
        self.provider = provider
        self.workspace_dir = workspace_dir
        self.config_dir = config_dir
        self.expected_style = expected_style


def _provider_id(context: Dict[str, Any]) -> str:
    provider = context.get("provider")
    if isinstance(provider, dict):
        provider_id = provider.get("id")
        if isinstance(provider_id, str) and provider_id.strip():
            return provider_id.strip()
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


def _tool_calls(context: Dict[str, Any]) -> list[dict[str, Any]]:
    provider_resp = context.get("providerResponse")
    if not isinstance(provider_resp, dict):
        return []
    meta = provider_resp.get("metadata")
    if not isinstance(meta, dict):
        return []
    tool_calls = meta.get("toolCalls")
    if isinstance(tool_calls, list):
        out: list[dict[str, Any]] = []
        for item in tool_calls:
            if isinstance(item, dict):
                out.append(item)
        return out
    return []


def _find_workspace_root(path: Path) -> Optional[Path]:
    # In case the agent runs `pwd` from a subdirectory, walk up a few levels to
    # find the workspace root that contains llmanspec/config.yaml.
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
    # Prefer reading from toolCalls because provider objects are not reliably serializable.
    # The Read toolcall for llmanspec/config.yaml is the most reliable signal.
    for tc in _tool_calls(context):
        ws = _maybe_workspace_from_config_read_toolcall(tc)
        if ws is not None:
            return ws

    for tc in _tool_calls(context):
        ws = _maybe_workspace_from_pwd_toolcall(tc)
        if ws is not None:
            return ws

    return None


def _parse_spec_style_from_config(config_text: str) -> Optional[str]:
    # Keep this lightweight; we don't want a YAML dependency inside promptfoo assertions.
    # Accept simple forms:
    #   spec_style: ison
    #   spec_style: "ison"
    m = re.search(
        r"^\s*spec_style\s*:\s*[\"']?([A-Za-z0-9_-]+)[\"']?\s*(?:#.*)?$",
        config_text,
        flags=re.MULTILINE,
    )
    if not m:
        return None
    style = m.group(1).strip().lower()
    if style in {"ison", "toon", "yaml"}:
        return style
    return None


def _infer_style_from_workspace(workspace_dir: Path) -> Optional[str]:
    config_path = workspace_dir / "llmanspec" / "config.yaml"
    if not config_path.exists():
        return None
    config_text = _read_text(config_path)
    return _parse_spec_style_from_config(config_text)


def _infer_config_dir(workspace_dir: Path, style: str) -> Optional[Path]:
    # Prefer runner-exported env vars.
    style_upper = style.upper()
    env_value = os.environ.get(f"SDD_CONFIGDIR_{style_upper}")
    if env_value:
        candidate = Path(env_value)
        if candidate.is_dir():
            return candidate

    # Fallback to runner layout: <work_dir>/{workspaces,configs}/{style}
    # workspace_dir is expected to be <work_dir>/workspaces/<style>
    if workspace_dir.parent.name == "workspaces":
        work_dir = workspace_dir.parent.parent
        candidate = work_dir / "configs" / style
        if candidate.is_dir():
            return candidate
    return None


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

    workspace_dir = _infer_workspace_dir(context)
    if workspace_dir is None:
        # Fallback (best-effort): some promptfoo contexts may omit toolCalls; in that case,
        # try mapping via provider id + runner exported env vars.
        expected_style = _select_style(provider_id)
        workdir_str, configdir_str = _workspace_paths_for(expected_style)
        if workdir_str and configdir_str:
            return GateContext(
                provider=provider_id,
                workspace_dir=Path(workdir_str),
                config_dir=Path(configdir_str),
                expected_style=expected_style,
            )
        raise RuntimeError(
            "Unable to infer workspace_dir from assertion context. "
            "Expected providerResponse.metadata.toolCalls to include `pwd` output or a config.yaml read. "
            f"provider_id={provider_id}"
        )

    expected_style = _infer_style_from_workspace(workspace_dir)
    if expected_style is None:
        raise RuntimeError(
            "Unable to infer spec_style from workspace llmanspec/config.yaml. "
            f"workspace_dir={workspace_dir} provider_id={provider_id}"
        )

    config_dir = _infer_config_dir(workspace_dir, expected_style)
    if config_dir is None:
        raise RuntimeError(
            "Unable to infer config_dir for hard gate. "
            f"Expected SDD_CONFIGDIR_{expected_style.upper()} or runner layout <work_dir>/configs/{expected_style}. "
            f"workspace_dir={workspace_dir} provider_id={provider_id}"
        )

    return GateContext(
        provider=provider_id,
        workspace_dir=workspace_dir,
        config_dir=config_dir,
        expected_style=expected_style,
    )


def _check_style_fences(gate: GateContext) -> Optional[str]:
    config_path = gate.workspace_dir / "llmanspec" / "config.yaml"
    if not config_path.exists():
        return f"Missing config: {config_path}"
    config_text = _read_text(config_path)
    parsed_style = _parse_spec_style_from_config(config_text)
    if parsed_style != gate.expected_style:
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
