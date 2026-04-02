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
  --model <alias>                 默认：sonnet（Claude Code SDK 模型别名）
  --max-turns <N>                 默认：18
  --runs <N>                      默认：1（独立 run 次数，每次新 seed 根目录）
  --repeat <N>                    透传给 promptfoo eval --repeat（注意：同一 run 内会复用 workspace）
  --judge <off|human|codex|claude> 默认：off（可选软评分；不替代硬门禁）
  --judge-grader <provider>       可选；judge=codex/claude 时覆盖 promptfoo --grader
  --llman-bin <path>              可选；覆盖 llman 可执行文件路径
  --max-concurrency <N>           透传给 promptfoo eval --max-concurrency
  --delay <ms>                    透传给 promptfoo eval --delay
  --no-cache                      透传给 promptfoo eval --no-cache
  --no-run                        只生成 workspaces + promptfoo 目录，不执行 promptfoo eval

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
REPEAT=""
MAX_CONCURRENCY=""
DELAY_MS=""
NO_CACHE="0"
NO_RUN="0"

JUDGE="off"
JUDGE_GRADER=""

LLMAN_BIN_OVERRIDE=""

CC_ACCOUNT=""
CC_CONFIG_DIR="${HOME}/.config/llman"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --model)
      MODEL="${2:-}"; shift 2;;
    --max-turns)
      MAX_TURNS="${2:-}"; shift 2;;
    --runs)
      RUNS="${2:-}"; shift 2;;
    --repeat)
      REPEAT="${2:-}"; shift 2;;
    --max-concurrency)
      MAX_CONCURRENCY="${2:-}"; shift 2;;
    --delay)
      DELAY_MS="${2:-}"; shift 2;;
    --no-cache)
      NO_CACHE="1"; shift 1;;
    --no-run)
      NO_RUN="1"; shift 1;;
    --judge)
      JUDGE="${2:-}"; shift 2;;
    --judge-grader)
      JUDGE_GRADER="${2:-}"; shift 2;;
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
if [[ -n "$REPEAT" ]]; then
  [[ "$REPEAT" =~ ^[0-9]+$ ]] || die "--repeat 必须是整数"
fi
if [[ -n "$MAX_CONCURRENCY" ]]; then
  [[ "$MAX_CONCURRENCY" =~ ^[0-9]+$ ]] || die "--max-concurrency 必须是整数"
fi
if [[ -n "$DELAY_MS" ]]; then
  [[ "$DELAY_MS" =~ ^[0-9]+$ ]] || die "--delay 必须是整数（ms）"
fi

case "$JUDGE" in
  off|human|codex|claude) ;;
  *) die "--judge 取值应为 off|human|codex|claude" ;;
esac

need_cmd git
need_cmd python3
need_cmd promptfoo

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
lines.append(f"# Promptfoo Summary\\n")
lines.append(f"- evalId: `{summary.get('evalId')}`\\n")
lines.append("")
lines.append("| provider | cases | ok | fail | err | turns(avg/max) | tokens(total) | cost(usd) | permission_denials |")
lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
for pid, vals in summary["providers"].items():
    avg = vals["avg_turns"]
    avg_s = f\"{avg:.2f}\" if isinstance(avg, (int, float)) else \"-\"
    lines.append(
        f\"| `{pid}` | {vals['cases']} | {vals['successes']} | {vals['failures']} | {vals['errors']} \"
        f\"| {avg_s}/{vals['num_turns_max']} \"
        f\"| {vals['tokens_total']} \"
        f\"| {vals['cost_usd']:.6f} \"
        f\"| {vals['permission_denials']} |\"
    )

with open(out_md_path, "w", encoding="utf-8") as f:
    f.write(\"\\n\".join(lines) + \"\\n\")
PY
}

run_one() {
  local run_idx="$1"

  local seed
  seed="$(python3 - <<'PY'
import os
print(os.urandom(4).hex())
PY
)"

  local work_dir="$REPO_ROOT/.tmp/sdd-claude-style-eval/${timestamp_utc}_${git_sha}_r${run_idx}_${seed}"
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

  local ws_ison="$workspaces_dir/ison"
  local ws_toon="$workspaces_dir/toon"
  local ws_yaml="$workspaces_dir/yaml"

  local cfg_ison="$configs_dir/ison"
  local cfg_toon="$configs_dir/toon"
  local cfg_yaml="$configs_dir/yaml"

  init_workspace "ison" "$ws_ison" "$cfg_ison"
  init_workspace "toon" "$ws_toon" "$cfg_toon"
  init_workspace "yaml" "$ws_yaml" "$cfg_yaml"

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
  fixture_src="$REPO_ROOT/agentdev/promptfoo/sdd_llmanspec_styles_v1"
  [[ -d "$fixture_src" ]] || die "找不到 promptfoo fixture：$fixture_src"
  cp -R "$fixture_src/." "$promptfoo_dir/"

  # Ensure claude-agent-sdk is resolvable from promptfoo_dir via node resolution.
  if [[ ! -e "$promptfoo_dir/node_modules" ]]; then
    ln -s "$REPO_ROOT/agentdev/promptfoo/node_modules" "$promptfoo_dir/node_modules"
  fi

  patch_promptfoo_fixture "$promptfoo_dir" "$ws_ison" "$ws_toon" "$ws_yaml" "$cfg_ison" "$cfg_toon" "$cfg_yaml" "$path_ison" "$path_toon" "$path_yaml"

  echo
  echo "== promptfoo validate config"
  (cd "$promptfoo_dir" && promptfoo validate config -c "$promptfoo_dir/promptfooconfig.yaml")

  if [[ "$NO_RUN" == "1" ]]; then
    echo
    echo "（跳过 promptfoo eval：因为传入了 --no-run）"
  else
    echo
    echo "== promptfoo eval"
    eval_args=(promptfoo eval --config "$promptfoo_dir/promptfooconfig.yaml" --output "$promptfoo_dir/results.json" --output "$promptfoo_dir/results.html")
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
      eval_args+=(--grader "${JUDGE_GRADER:-openai:chat:gpt-5.2}")
    fi
    if [[ "$JUDGE" == "claude" ]]; then
      eval_args+=(--grader "${JUDGE_GRADER:-anthropic:messages:claude-3-5-sonnet-latest}")
    fi
    (cd "$promptfoo_dir" && "${eval_args[@]}")
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
}

if (( RUNS < 1 )); then
  die "--runs 必须 >= 1"
fi

for i in $(seq 1 "$RUNS"); do
  run_one "$i"
done
