#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
用法：
  bash agentdev/promptfoo/run-sdd-claude-style-eval.sh [options]

兼容入口：
  bash scripts/sdd-claude-style-eval.sh [options]

说明：
  - 通过 Promptfoo 驱动 Claude Code agent（anthropic:claude-agent-sdk）
  - 在每次 run 的临时根目录中创建 3 个隔离 workspace（ison/toon/yaml）
  - 允许真实写文件/执行命令（bypassPermissions），并用 Python assertions 做硬门禁：
    `llman sdd validate --all --strict --no-interactive`
  - 每个 workspace 初始化为 git repo，并输出 meta 快照（git log/diff/status + validate）

依赖：
  - promptfoo（建议全局安装）
  - python3
  - git
  - llman（优先使用仓库内 `target/debug/llman`；否则使用 PATH 中的 `llman`）
  - agentdev/promptfoo/node_modules/@anthropic-ai/claude-agent-sdk（首次需安装）

常用选项：
  --fixture <v1|v2>               默认：v1（v2 为 format-sensitive；runner 预置 baseline + 生成 batch aggregate）
  --model <alias>                 默认：sonnet（Claude Code SDK 模型别名）
  --max-turns <N>                 默认：18
  --runs <N>                      默认：1（独立 run 次数，每次新 seed 根目录）
  --repeat <N>                    透传给 promptfoo eval --repeat（注意：同一 run 内会复用 workspace）
  --api-key-env <VAR>             可选；从指定环境变量读取 API key（默认自动探测）
  --judge <off|human|codex|claude> 默认：off（可选软评分；不替代硬门禁）
  --judge-grader <provider>       可选；judge=codex/claude 时覆盖 promptfoo --grader
  --eval-retries <N>              默认：2（promptfoo eval 失败时最多重试 N 次；总尝试次数=1+N）
  --llman-bin <path>              可选；覆盖 llman 可执行文件路径
  --max-concurrency <N>           透传给 promptfoo eval --max-concurrency
  --delay <ms>                    透传给 promptfoo eval --delay
  --no-cache                      透传给 promptfoo eval --no-cache
  --no-run                        只生成 workspaces + promptfoo 目录，不执行 promptfoo eval
  --ui                            评测结束后启动 Promptfoo Web UI（promptfoo view -y；会阻塞）
  --ui-port <N>                   Web UI 端口（默认：15500）

Claude Code account 注入（敏感：请勿直接运行 env 子命令输出到终端）：
  --cc-account <name>             例如：glm-lite-150 / glm-lite-156
  --cc-config-dir <path>          默认：~/.config/llman
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "缺少依赖命令：$1"
}

MODEL="sonnet"
MAX_TURNS="18"
RUNS="1"
FIXTURE="v1"
REPEAT=""
MAX_CONCURRENCY=""
DELAY_MS=""
NO_CACHE="0"
NO_RUN="0"
OPEN_UI="0"
UI_PORT="15500"

JUDGE="off"
JUDGE_GRADER=""
EVAL_RETRIES="2"

LLMAN_BIN_OVERRIDE=""
API_KEY_ENV=""

CC_ACCOUNT=""
CC_CONFIG_DIR="${HOME}/.config/llman"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --fixture)
      FIXTURE="${2:-}"; shift 2;;
    --model)
      MODEL="${2:-}"; shift 2;;
    --max-turns)
      MAX_TURNS="${2:-}"; shift 2;;
    --runs)
      RUNS="${2:-}"; shift 2;;
    --repeat)
      REPEAT="${2:-}"; shift 2;;
    --api-key-env)
      API_KEY_ENV="${2:-}"; shift 2;;
    --max-concurrency)
      MAX_CONCURRENCY="${2:-}"; shift 2;;
    --delay)
      DELAY_MS="${2:-}"; shift 2;;
    --no-cache)
      NO_CACHE="1"; shift 1;;
    --no-run)
      NO_RUN="1"; shift 1;;
    --ui)
      OPEN_UI="1"; shift 1;;
    --ui-port)
      UI_PORT="${2:-}"; shift 2;;
    --judge)
      JUDGE="${2:-}"; shift 2;;
    --judge-grader)
      JUDGE_GRADER="${2:-}"; shift 2;;
    --eval-retries)
      EVAL_RETRIES="${2:-}"; shift 2;;
    --llman-bin)
      LLMAN_BIN_OVERRIDE="${2:-}"; shift 2;;
    --cc-account)
      CC_ACCOUNT="${2:-}"; shift 2;;
    --cc-config-dir)
      CC_CONFIG_DIR="${2:-}"; shift 2;;
    *)
      die "未知参数：$1（使用 --help 查看）"
      ;;
  esac
done

[[ -n "$MODEL" ]] || die "--model 不能为空"
[[ "$MAX_TURNS" =~ ^[0-9]+$ ]] || die "--max-turns 必须是整数"
[[ "$RUNS" =~ ^[0-9]+$ ]] || die "--runs 必须是整数"
case "$FIXTURE" in
  v1|v2) ;;
  *) die "--fixture 取值应为 v1|v2" ;;
esac
if [[ -n "$REPEAT" ]]; then
  [[ "$REPEAT" =~ ^[0-9]+$ ]] || die "--repeat 必须是整数"
fi
if [[ -n "$MAX_CONCURRENCY" ]]; then
  [[ "$MAX_CONCURRENCY" =~ ^[0-9]+$ ]] || die "--max-concurrency 必须是整数"
fi
if [[ -n "$DELAY_MS" ]]; then
  [[ "$DELAY_MS" =~ ^[0-9]+$ ]] || die "--delay 必须是整数（ms）"
fi
if [[ -n "$UI_PORT" ]]; then
  [[ "$UI_PORT" =~ ^[0-9]+$ ]] || die "--ui-port 必须是整数"
fi
[[ "$EVAL_RETRIES" =~ ^[0-9]+$ ]] || die "--eval-retries 必须是整数"

case "$JUDGE" in
  off|human|codex|claude) ;;
  *) die "--judge 取值应为 off|human|codex|claude" ;;
esac

need_cmd git
need_cmd python3
need_cmd node

PROMPTFOO_CMD=(promptfoo)
if ! promptfoo --version >/dev/null 2>&1; then
  # Some promptfoo installs may break due to native better-sqlite3 bindings. Fallback to a working
  # pnpm global store entry if possible (doesn't require re-installing in the repo).
  if command -v pnpm >/dev/null 2>&1; then
    global_root="$(pnpm root -g 2>/dev/null || true)"
    global_store=""
    if [[ -n "$global_root" ]]; then
      global_store="$(dirname "$global_root")/.pnpm"
    fi

    promptfoo_main_js=""
    if [[ -n "$global_store" && -d "$global_store" ]]; then
      while IFS= read -r dir; do
        candidate="$dir/node_modules/promptfoo/dist/src/main.js"
        if [[ -f "$candidate" ]] && node "$candidate" --version >/dev/null 2>&1; then
          promptfoo_main_js="$candidate"
          break
        fi
      done < <(ls -d "$global_store"/promptfoo@* 2>/dev/null | sort -r || true)
    fi

    if [[ -n "$promptfoo_main_js" ]]; then
      PROMPTFOO_CMD=(node "$promptfoo_main_js")
      echo "== promptfoo: fallback to pnpm global store entry"
    else
      die "promptfoo 无法运行（可能是 better-sqlite3 原生依赖问题）。请安装/修复 promptfoo，或使用 Docker runner。"
    fi
  else
    die "promptfoo 无法运行，且未检测到 pnpm 可用于回退。请安装/修复 promptfoo，或使用 Docker runner。"
  fi
fi

if [[ -n "$LLMAN_BIN_OVERRIDE" ]]; then
  LLMAN_BIN="$LLMAN_BIN_OVERRIDE"
else
  # Prefer repo-built llman (required for up-to-date templates/features in this repo).
  if [[ ! -x "$REPO_ROOT/target/debug/llman" ]] && command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build -q)
  fi

  if [[ -x "$REPO_ROOT/target/debug/llman" ]]; then
    LLMAN_BIN="$REPO_ROOT/target/debug/llman"
  else
    LLMAN_BIN="$(command -v llman || true)"
  fi
fi

[[ -n "${LLMAN_BIN:-}" ]] || die "找不到 llman（建议在仓库根目录先运行 cargo build）"
[[ -x "$LLMAN_BIN" ]] || die "llman 不可执行：$LLMAN_BIN"

deps_dir="$REPO_ROOT/agentdev/promptfoo/node_modules/@anthropic-ai/claude-agent-sdk"
if [[ ! -d "$deps_dir" ]]; then
  cat <<EOF >&2
Error: 缺少 Claude Agent SDK 依赖：$deps_dir

请先在本机安装依赖（不会提交 node_modules）：
  pnpm -C $REPO_ROOT/agentdev/promptfoo install

或使用 Docker runner（见 agentdev/docker）。
EOF
  exit 1
fi

git_sha="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
timestamp_utc="$(date -u +%Y-%m-%dT%H%M%SZ)"

resolve_promptfoo_anthropic_key_source_env() {
  # `anthropic:claude-agent-sdk` provider only checks ANTHROPIC_API_KEY (not auth tokens) when
  # promptfoo pre-checks for missing keys. We therefore copy from a configured env var when needed.
  if [[ -n "$API_KEY_ENV" ]]; then
    if [[ -n "${!API_KEY_ENV:-}" ]]; then
      echo "$API_KEY_ENV"
      return 0
    fi
    die "--api-key-env 指定的环境变量未设置或为空：$API_KEY_ENV"
  fi

  local candidates=(
    "ANTHROPIC_API_KEY"
    "ANTHROPIC_AUTH_TOKEN"
    "GLM_API_KEY"
    "CLAUDE_API_KEY"
    "CLAUDE_CODE_API_KEY"
    "CLAUDE_CODE_TOKEN"
  )
  local v
  for v in "${candidates[@]}"; do
    if [[ -n "${!v:-}" ]]; then
      echo "$v"
      return 0
    fi
  done
  return 1
}

ensure_promptfoo_anthropic_api_key() {
  if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
    return 0
  fi

  local source_env
  source_env="$(resolve_promptfoo_anthropic_key_source_env || true)"
  if [[ -z "$source_env" ]]; then
    cat <<EOF >&2
Error: 缺少 Promptfoo/Claude Agent SDK 所需的 `ANTHROPIC_API_KEY`。

说明：
- promptfoo 的 `anthropic:claude-agent-sdk` provider 在启动前会预检 `ANTHROPIC_API_KEY`
- 但 Claude Code account env 注入通常提供 `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_BASE_URL`

解决方式（二选一）：
1) 显式提供 API key：export ANTHROPIC_API_KEY=...
2) 或用 --api-key-env <VAR> 指定要复制到 ANTHROPIC_API_KEY 的环境变量（例如 ANTHROPIC_AUTH_TOKEN / GLM_API_KEY）
EOF
    exit 1
  fi

  export ANTHROPIC_API_KEY="${!source_env}"
  [[ -n "${ANTHROPIC_API_KEY:-}" ]] || die "环境变量为空：$source_env"
  echo "== promptfoo api key source: $source_env -> ANTHROPIC_API_KEY"
}

init_workspace() {
  local style="$1"
  local workspace_dir="$2"
  local config_dir="$3"

  mkdir -p "$workspace_dir" "$config_dir"

  echo "== init workspace ($style): $workspace_dir"
  (cd "$workspace_dir" && LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd init "$workspace_dir" --lang en >/dev/null)

  if [[ "$style" != "ison" ]]; then
    (cd "$workspace_dir" && LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd convert --to "$style" --project >/dev/null)
  fi

  mkdir -p "$workspace_dir/.llman-bin"
  cp "$LLMAN_BIN" "$workspace_dir/.llman-bin/llman"
  chmod +x "$workspace_dir/.llman-bin/llman"

  git -C "$workspace_dir" init -q
  git -C "$workspace_dir" config user.email "agentdev@example.com"
  git -C "$workspace_dir" config user.name "agentdev"
  git -C "$workspace_dir" add -A
  git -C "$workspace_dir" commit -qm "baseline"
}

seed_workspace_v2() {
  local style="$1"
  local workspace_dir="$2"
  local config_dir="$3"

  echo "== seed baseline (v2) ($style): $workspace_dir"

  (
    cd "$workspace_dir"

    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec skeleton "eval-style-cap" --force >/dev/null
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec add-requirement "eval-style-cap" "REQ_STYLE_0" \
      --title "Baseline context" \
      --statement "The evaluation MUST be format-sensitive across styles." >/dev/null
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec add-scenario "eval-style-cap" "REQ_STYLE_0" "s1" \
      --when "the suite runs" \
      --then "it compares style-specific costs" >/dev/null

    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec add-requirement "eval-style-cap" "REQ_STYLE_1" \
      --title "Marker edit target" \
      --statement "The evaluation MUST validate under strict mode. (TODO_V2_EDIT_MAIN)" >/dev/null
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec add-scenario "eval-style-cap" "REQ_STYLE_1" "s1" \
      --when "validation runs" \
      --then "it passes in strict mode" >/dev/null

    mkdir -p "llmanspec/changes/eval-style-change"
    cat >"llmanspec/changes/eval-style-change/proposal.md" <<'EOF'
# eval-style-change

This change exists to create a delta spec so the evaluation can measure read/edit cost across styles.
EOF

    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd delta skeleton "eval-style-change" "eval-style-cap" --force >/dev/null
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd delta add-op "eval-style-change" "eval-style-cap" add_requirement "REQ_STYLE_2" \
      --title "Delta marker edit target" \
      --statement "The change MUST include at least one op scenario. (TODO_V2_EDIT_DELTA)" >/dev/null
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd delta add-scenario "eval-style-change" "eval-style-cap" "REQ_STYLE_2" "s1" \
      --when "validation runs" \
      --then "it passes in strict mode" >/dev/null

    git add -A
    git commit -qm "seed: baseline specs (v2)"
  )
}

patch_promptfoo_fixture() {
  local promptfoo_dir="$1"
  local workdir_ison="$2"
  local workdir_toon="$3"
  local workdir_yaml="$4"
  local configdir_ison="$5"
  local configdir_toon="$6"
  local configdir_yaml="$7"
  local path_ison="$8"
  local path_toon="$9"
  local path_yaml="${10}"

  local config_path="$promptfoo_dir/promptfooconfig.yaml"
  [[ -f "$config_path" ]] || die "找不到 promptfoo config：$config_path"

  python3 - \
    "$config_path" \
    "$MODEL" \
    "$MAX_TURNS" \
    "$workdir_ison" \
    "$workdir_toon" \
    "$workdir_yaml" \
    "$configdir_ison" \
    "$configdir_toon" \
    "$configdir_yaml" \
    "$path_ison" \
    "$path_toon" \
    "$path_yaml" \
    "$JUDGE" <<'PY'
import sys

(
    path,
    model,
    max_turns,
    workdir_ison,
    workdir_toon,
    workdir_yaml,
    configdir_ison,
    configdir_toon,
    configdir_yaml,
    path_ison,
    path_toon,
    path_yaml,
    judge,
) = sys.argv[1:]

max_turns = int(max_turns)

with open(path, "r", encoding="utf-8") as f:
    text = f.read()

replacements = {
    "__MODEL__": model,
    "__MAX_TURNS__": str(max_turns),
    "__WORKDIR_ISON__": workdir_ison,
    "__WORKDIR_TOON__": workdir_toon,
    "__WORKDIR_YAML__": workdir_yaml,
    "__CONFIGDIR_ISON__": configdir_ison,
    "__CONFIGDIR_TOON__": configdir_toon,
    "__CONFIGDIR_YAML__": configdir_yaml,
    "__PATH_ISON__": path_ison,
    "__PATH_TOON__": path_toon,
    "__PATH_YAML__": path_yaml,
}

for needle, value in replacements.items():
    if needle not in text:
        raise SystemExit(f"Missing placeholder: {needle} in {path}")
    text = text.replace(needle, value)

judge_marker = "# __JUDGE_ASSERT_BLOCK__ (patched by runner when enabled)"
if judge_marker not in text:
    raise SystemExit(f"Missing judge marker line in {path}")

if judge in ("codex", "claude"):
    block = """- type: llm-rubric
      value: |
        {{ rubric }}
      threshold: 0.75
"""
    # Preserve indentation (4 spaces) from the marker line position.
    text = text.replace("    " + judge_marker, "    " + block.rstrip("\n"))
else:
    # Remove marker entirely (judge off/human)
    text = text.replace("    " + judge_marker + "\n", "")

with open(path, "w", encoding="utf-8") as f:
    f.write(text)
PY
}

write_meta_workspace() {
  local style="$1"
  local workspace_dir="$2"
  local config_dir="$3"
  local out_dir="$4"

  mkdir -p "$out_dir"
  git -C "$workspace_dir" status --porcelain=v1 > "$out_dir/git.status.txt" || true
  git -C "$workspace_dir" log --oneline --decorate --graph --max-count 50 > "$out_dir/git.log.txt" || true
  git -C "$workspace_dir" diff > "$out_dir/git.diff.txt" || true

  (
    cd "$workspace_dir" \
      && LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd validate --all --strict --no-interactive
  ) > "$out_dir/validate.txt" 2>&1 || true
}

reset_workspace_to_sha() {
  local style="$1"
  local workspace_dir="$2"
  local sha="$3"

  echo "== reset workspace ($style) -> $sha"
  git -C "$workspace_dir" reset --hard "$sha" >/dev/null
  git -C "$workspace_dir" clean -fdx >/dev/null
}

summarize_results() {
  local results_json="$1"
  local out_json="$2"
  local out_md="$3"

  python3 - "$results_json" "$out_json" "$out_md" <<'PY'
from __future__ import annotations

import json
import sys
from collections import defaultdict

results_path, out_json_path, out_md_path = sys.argv[1:4]

with open(results_path, "r", encoding="utf-8") as f:
    data = json.load(f)

rows = data.get("results", {}).get("results", []) or []
agg = defaultdict(
    lambda: {
        "cases": 0,
        "successes": 0,
        "failures": 0,
        "errors": 0,
        "cost_usd": 0.0,
        "tokens_prompt": 0,
        "tokens_completion": 0,
        "tokens_total": 0,
        "num_turns_sum": 0,
        "num_turns_max": 0,
        "permission_denials": 0,
    }
)

def to_int(value):
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return None

for row in rows:
    provider = row.get("provider") or {}
    provider_id = provider.get("id") if isinstance(provider, dict) else str(provider)

    entry = agg[provider_id]
    entry["cases"] += 1

    if row.get("error"):
        entry["errors"] += 1
    elif row.get("success") is True:
        entry["successes"] += 1
    else:
        entry["failures"] += 1

    cost = row.get("cost")
    if isinstance(cost, (int, float)):
        entry["cost_usd"] += float(cost)

    resp = row.get("response") or {}
    tu = resp.get("tokenUsage") or {}
    for k, out_key in [("prompt", "tokens_prompt"), ("completion", "tokens_completion"), ("total", "tokens_total")]:
        v = to_int(tu.get(k))
        if v is not None:
            entry[out_key] += v

    meta = resp.get("metadata") or {}
    num_turns = to_int(meta.get("numTurns"))
    if num_turns is not None:
        entry["num_turns_sum"] += num_turns
        entry["num_turns_max"] = max(entry["num_turns_max"], num_turns)

    denials = meta.get("permissionDenials")
    if isinstance(denials, list):
        entry["permission_denials"] += len(denials)

summary = {
    "evalId": data.get("evalId"),
    "providers": {
        pid: {
            **vals,
            "avg_turns": (vals["num_turns_sum"] / vals["cases"]) if vals["cases"] else None,
        }
        for pid, vals in sorted(agg.items(), key=lambda kv: kv[0])
    },
}

with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)

lines = []
lines.append("# Promptfoo Summary")
lines.append(f"- evalId: `{summary.get('evalId')}`")
lines.append("")
lines.append("| provider | cases | ok | fail | err | turns(avg/max) | tokens(total) | cost(usd) | permission_denials |")
lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
for pid, vals in summary["providers"].items():
    avg = vals["avg_turns"]
    avg_s = f"{avg:.2f}" if isinstance(avg, (int, float)) else "-"
    lines.append(
        f"| `{pid}` | {vals['cases']} | {vals['successes']} | {vals['failures']} | {vals['errors']} "
        f"| {avg_s}/{vals['num_turns_max']} "
        f"| {vals['tokens_total']} "
        f"| {vals['cost_usd']:.6f} "
        f"| {vals['permission_denials']} |"
    )

with open(out_md_path, "w", encoding="utf-8") as f:
    f.write("\n".join(lines) + "\n")
PY
}

aggregate_batch_results() {
  local batch_dir="$1"
  local out_json="$2"
  local out_md="$3"

  python3 - "$batch_dir" "$out_json" "$out_md" "$FIXTURE" "$MODEL" "$MAX_TURNS" "$RUNS" "$REPEAT" "$JUDGE" <<'PY'
from __future__ import annotations

import json
import math
import os
import sys
from dataclasses import dataclass
from glob import glob
from typing import Any, Dict, List, Optional

(
    batch_dir,
    out_json_path,
    out_md_path,
    fixture,
    model,
    max_turns,
    runs,
    repeat,
    judge,
) = sys.argv[1:]

max_turns_i = int(max_turns)
runs_i = int(runs)
repeat_i: Optional[int] = int(repeat) if repeat else None


def select_style(provider_id: str) -> str:
    lowered = provider_id.lower()
    if "ison" in lowered:
        return "ison"
    if "toon" in lowered:
        return "toon"
    if "yaml" in lowered:
        return "yaml"
    return "unknown"


def to_int(value: Any) -> Optional[int]:
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return None


def percentile(sorted_values: List[float], p: float) -> Optional[float]:
    if not sorted_values:
        return None
    if p <= 0:
        return float(sorted_values[0])
    if p >= 1:
        return float(sorted_values[-1])
    idx = int(math.ceil(p * len(sorted_values)) - 1)
    idx = max(0, min(idx, len(sorted_values) - 1))
    return float(sorted_values[idx])


def stats(values: List[float]) -> Dict[str, Any]:
    values_sorted = sorted(values)
    n = len(values_sorted)
    if n == 0:
        return {"n": 0, "mean": None, "median": None, "p90": None}
    mean = sum(values_sorted) / n
    if n % 2 == 1:
        median = float(values_sorted[n // 2])
    else:
        median = (values_sorted[n // 2 - 1] + values_sorted[n // 2]) / 2.0
    return {
        "n": n,
        "mean": mean,
        "median": median,
        "p90": percentile(values_sorted, 0.90),
    }


@dataclass
class Row:
    style: str
    success: bool
    error: bool
    tokens_total: Optional[int]
    turns: Optional[int]
    cost_usd: Optional[float]


rows: List[Row] = []

result_paths = sorted(glob(os.path.join(batch_dir, "runs", "*", "promptfoo", "results.json")))
for path in result_paths:
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except Exception:
        continue

    items = data.get("results", {}).get("results", []) or []
    for item in items:
        provider = item.get("provider") or {}
        provider_id = provider.get("id") if isinstance(provider, dict) else str(provider)
        style = select_style(provider_id)

        is_error = bool(item.get("error"))
        is_success = (item.get("success") is True) and not is_error

        cost = item.get("cost")
        cost_f = float(cost) if isinstance(cost, (int, float)) else None

        resp = item.get("response") or {}
        tu = resp.get("tokenUsage") or {}
        tokens_total = to_int(tu.get("total"))

        meta = resp.get("metadata") or {}
        turns = to_int(meta.get("numTurns"))

        rows.append(
            Row(
                style=style,
                success=is_success,
                error=is_error,
                tokens_total=tokens_total,
                turns=turns,
                cost_usd=cost_f,
            )
        )

styles = ["ison", "toon", "yaml"]
style_stats: Dict[str, Any] = {}
for style in styles:
    style_rows = [r for r in rows if r.style == style]
    total = len(style_rows)
    ok = sum(1 for r in style_rows if r.success)
    err = sum(1 for r in style_rows if r.error)
    fail = total - ok - err

    tokens_vals = [float(r.tokens_total) for r in style_rows if r.tokens_total is not None]
    turns_vals = [float(r.turns) for r in style_rows if r.turns is not None]
    cost_vals = [float(r.cost_usd) for r in style_rows if r.cost_usd is not None]

    style_stats[style] = {
        "cases": total,
        "successes": ok,
        "failures": fail,
        "errors": err,
        "pass_rate": (ok / total) if total else None,
        "tokens_total": stats(tokens_vals),
        "turns": stats(turns_vals),
        "cost_usd": stats(cost_vals),
    }

summary = {
    "batch_dir": batch_dir,
    "config": {
        "fixture": fixture,
        "model": model,
        "max_turns": max_turns_i,
        "runs": runs_i,
        "repeat": repeat_i,
        "judge": judge,
    },
    "inputs": {
        "result_paths": result_paths,
    },
    "styles": style_stats,
}

with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)

lines = []
lines.append("# Batch Aggregate Summary\n")
lines.append(f"- batch_dir: `{batch_dir}`\n")
lines.append(f"- fixture: `{fixture}`\n")
lines.append(f"- model: `{model}`\n")
lines.append(f"- max_turns: `{max_turns_i}`\n")
lines.append(f"- runs: `{runs_i}`\n")
if repeat_i is not None:
    lines.append(f"- repeat: `{repeat_i}`\n")
lines.append(f"- judge: `{judge}`\n")
lines.append("")
lines.append("| style | cases | pass_rate | tokens(mean/med/p90) | turns(mean/med/p90) | cost_usd(mean/med/p90) |")
lines.append("|---|---:|---:|---:|---:|---:|")
for style in styles:
    s = style_stats[style]
    pr = s["pass_rate"]
    pr_s = f"{pr:.2%}" if isinstance(pr, (int, float)) else "-"

    def fmt(triple: Dict[str, Any]) -> str:
        if triple.get("n", 0) == 0:
            return "-"
        return f"{triple['mean']:.2f}/{triple['median']:.2f}/{triple['p90']:.2f}"

    lines.append(
        f"| `{style}` | {s['cases']} | {pr_s} | {fmt(s['tokens_total'])} | {fmt(s['turns'])} | {fmt(s['cost_usd'])} |"
    )

with open(out_md_path, "w", encoding="utf-8") as f:
    f.write("\n".join(lines) + "\n")
PY
}

LAST_PROMPTFOO_DIR=""
LAST_BATCH_DIR=""

run_one() {
  local run_idx="$1"

  local seed
  seed="$(python3 - <<'PY'
import os
print(os.urandom(4).hex())
PY
)"

  local work_dir="$BATCH_DIR/runs/r${run_idx}_${seed}"
  local workspaces_dir="$work_dir/workspaces"
  local configs_dir="$work_dir/configs"
  local meta_dir="$work_dir/meta"
  local promptfoo_dir="$work_dir/promptfoo"

  mkdir -p "$workspaces_dir" "$configs_dir" "$meta_dir" "$promptfoo_dir"

  echo
  echo "== work_dir: $work_dir"

  if [[ -n "$CC_ACCOUNT" ]]; then
    echo "== source claude-code account env: $CC_ACCOUNT"
    # WARNING: do NOT print env exports (sensitive). `source <(...)` keeps it out of stdout.
    source <("$LLMAN_BIN" --config-dir "$CC_CONFIG_DIR" x claude-code account env "$CC_ACCOUNT")
  fi
  if [[ "$NO_RUN" != "1" ]]; then
    ensure_promptfoo_anthropic_api_key
  fi

  local ws_ison="$workspaces_dir/ison"
  local ws_toon="$workspaces_dir/toon"
  local ws_yaml="$workspaces_dir/yaml"

  local cfg_ison="$configs_dir/ison"
  local cfg_toon="$configs_dir/toon"
  local cfg_yaml="$configs_dir/yaml"

  init_workspace "ison" "$ws_ison" "$cfg_ison"
  init_workspace "toon" "$ws_toon" "$cfg_toon"
  init_workspace "yaml" "$ws_yaml" "$cfg_yaml"

  if [[ "$FIXTURE" == "v2" ]]; then
    echo
    echo "== seed baseline (v2)"
    seed_workspace_v2 "ison" "$ws_ison" "$cfg_ison"
    seed_workspace_v2 "toon" "$ws_toon" "$cfg_toon"
    seed_workspace_v2 "yaml" "$ws_yaml" "$cfg_yaml"
  fi

  local baseline_sha_ison
  local baseline_sha_toon
  local baseline_sha_yaml
  baseline_sha_ison="$(git -C "$ws_ison" rev-parse HEAD)"
  baseline_sha_toon="$(git -C "$ws_toon" rev-parse HEAD)"
  baseline_sha_yaml="$(git -C "$ws_yaml" rev-parse HEAD)"

  local path_ison="$ws_ison/.llman-bin:$PATH"
  local path_toon="$ws_toon/.llman-bin:$PATH"
  local path_yaml="$ws_yaml/.llman-bin:$PATH"

  # Export for Python assertions (executed in promptfoo process env, not provider env).
  export SDD_WORKDIR_ISON="$ws_ison"
  export SDD_WORKDIR_TOON="$ws_toon"
  export SDD_WORKDIR_YAML="$ws_yaml"
  export SDD_CONFIGDIR_ISON="$cfg_ison"
  export SDD_CONFIGDIR_TOON="$cfg_toon"
  export SDD_CONFIGDIR_YAML="$cfg_yaml"

  echo
  echo "== prepare promptfoo fixture"
  fixture_src="$REPO_ROOT/agentdev/promptfoo/sdd_llmanspec_styles_${FIXTURE}"
  [[ -d "$fixture_src" ]] || die "找不到 promptfoo fixture：$fixture_src"
  cp -R "$fixture_src/." "$promptfoo_dir/"

  # Ensure claude-agent-sdk is resolvable from promptfoo_dir via node resolution.
  if [[ ! -e "$promptfoo_dir/node_modules" ]]; then
    ln -s "$REPO_ROOT/agentdev/promptfoo/node_modules" "$promptfoo_dir/node_modules"
  fi

  patch_promptfoo_fixture "$promptfoo_dir" "$ws_ison" "$ws_toon" "$ws_yaml" "$cfg_ison" "$cfg_toon" "$cfg_yaml" "$path_ison" "$path_toon" "$path_yaml"

  echo
  echo "== promptfoo validate config"
  (cd "$promptfoo_dir" && "${PROMPTFOO_CMD[@]}" validate config -c "$promptfoo_dir/promptfooconfig.yaml")

  LAST_PROMPTFOO_DIR="$promptfoo_dir"

  local eval_exit="0"
  if [[ "$NO_RUN" == "1" ]]; then
    echo
    echo "（跳过 promptfoo eval：因为传入了 --no-run）"
  else
    echo
    echo "== promptfoo eval"
    eval_args=("${PROMPTFOO_CMD[@]}" eval --config "$promptfoo_dir/promptfooconfig.yaml" --output "$promptfoo_dir/results.json" --output "$promptfoo_dir/results.html")
    if [[ -n "$REPEAT" ]]; then
      eval_args+=(--repeat "$REPEAT")
    fi
    if [[ -n "$MAX_CONCURRENCY" ]]; then
      eval_args+=(--max-concurrency "$MAX_CONCURRENCY")
    fi
    if [[ -n "$DELAY_MS" ]]; then
      eval_args+=(--delay "$DELAY_MS")
    fi
    if [[ "$NO_CACHE" == "1" ]]; then
      eval_args+=(--no-cache)
    fi
    if [[ "$JUDGE" == "codex" ]]; then
      eval_args+=(--grader "${JUDGE_GRADER:-openai:chat:gpt-5.4-mini}")
    fi
    if [[ "$JUDGE" == "claude" ]]; then
      eval_args+=(--grader "${JUDGE_GRADER:-anthropic:messages:claude-3-5-sonnet-latest}")
    fi

    max_attempts="$((EVAL_RETRIES + 1))"
    for attempt in $(seq 1 "$max_attempts"); do
      if (( attempt > 1 )); then
        echo
        echo "== retrying promptfoo eval (attempt $attempt/$max_attempts)"
        reset_workspace_to_sha "ison" "$ws_ison" "$baseline_sha_ison"
        reset_workspace_to_sha "toon" "$ws_toon" "$baseline_sha_toon"
        reset_workspace_to_sha "yaml" "$ws_yaml" "$baseline_sha_yaml"
        # Small backoff to avoid immediate rate-limit loops.
        sleep 2
      fi

      if (cd "$promptfoo_dir" && "${eval_args[@]}"); then
        eval_exit="0"
        break
      fi

      eval_exit="$?"
      echo "!! promptfoo eval failed (exit=$eval_exit) (attempt $attempt/$max_attempts)" >&2
    done

    if [[ "$eval_exit" != "0" ]]; then
      echo "!! promptfoo eval failed after $max_attempts attempts. Continuing to write meta snapshots." >&2
    fi
  fi

  echo
  echo "== meta snapshots"
  write_meta_workspace "ison" "$ws_ison" "$cfg_ison" "$meta_dir/ison"
  write_meta_workspace "toon" "$ws_toon" "$cfg_toon" "$meta_dir/toon"
  write_meta_workspace "yaml" "$ws_yaml" "$cfg_yaml" "$meta_dir/yaml"

  if [[ -f "$promptfoo_dir/results.json" ]]; then
    echo
    echo "== summarize results"
    summarize_results "$promptfoo_dir/results.json" "$meta_dir/summary.json" "$meta_dir/summary.md"
    echo "summary: $meta_dir/summary.md"
  fi

  echo
  echo "== done"
  echo "promptfoo_dir: $promptfoo_dir"
  echo "meta_dir:      $meta_dir"

  return "$eval_exit"
}

if (( RUNS < 1 )); then
  die "--runs 必须 >= 1"
fi

batch_seed="$(python3 - <<'PY'
import os
print(os.urandom(4).hex())
PY
)"
BATCH_DIR="$REPO_ROOT/.tmp/sdd-claude-style-eval/${timestamp_utc}_${git_sha}_${FIXTURE}_b${batch_seed}"
mkdir -p "$BATCH_DIR/meta" "$BATCH_DIR/runs"
echo "== batch_dir: $BATCH_DIR"
LAST_BATCH_DIR="$BATCH_DIR"

overall_exit="0"
for i in $(seq 1 "$RUNS"); do
  if ! run_one "$i"; then
    overall_exit="1"
  fi
done

if (( RUNS >= 2 )) && [[ "$NO_RUN" != "1" ]]; then
  aggregate_out_json="$BATCH_DIR/meta/aggregate.json"
  aggregate_out_md="$BATCH_DIR/meta/aggregate.md"
  if ls "$BATCH_DIR"/runs/*/promptfoo/results.json >/dev/null 2>&1; then
    echo
    echo "== aggregate batch results"
    aggregate_batch_results "$BATCH_DIR" "$aggregate_out_json" "$aggregate_out_md"
    echo "aggregate: $aggregate_out_md"
  fi
fi

if [[ "$OPEN_UI" == "1" && -n "$LAST_PROMPTFOO_DIR" ]]; then
  echo
  echo "== promptfoo view (UI)"
  ui_pid=""
  trap 'if [[ -n "$ui_pid" ]]; then echo; echo "== stopping promptfoo UI"; kill "$ui_pid" 2>/dev/null || true; fi' INT
  set +e
  "${PROMPTFOO_CMD[@]}" view -y --port "$UI_PORT" "$LAST_PROMPTFOO_DIR" &
  ui_pid="$!"
  wait "$ui_pid"
  set -e
  trap - INT
fi

exit "$overall_exit"
