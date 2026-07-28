#!/usr/bin/env bash
set -euo pipefail

# Skill-gate evaluation runner: measures SKILL.md template quality by
# rendering the skill into an agentic prompt + driving a real task in a sandbox,
# with `llman sdd validate --all --strict` as the hard gate.
#
# Combines (from existing suites):
#   - sdd_apply_v1 / run-sdd-prompts-eval.sh: render SKILL.md into the prompt
#   - sdd_llmanspec_styles_v1 / run-sdd-claude-style-eval.sh: sandbox + hard gate + batch aggregation
# Dimension is baseline (previous template snapshot) vs candidate (current workspace).

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
用法：
  bash agentdev/promptfoo/run-sdd-skill-gate-eval.sh [options]

兼容入口：
  bash scripts/sdd-skill-gate-eval.sh [options]

说明：
  - 通过 Promptfoo 驱动 Claude Code agent（anthropic:claude-agent-sdk）
  - 渲染指定 SDD skill 的 SKILL.md 进 agent prompt（system 段）
  - 2 个隔离 workspace：baseline（上一版模板快照）vs candidate（当前工作区模板）
  - 每个 workspace 预置一个 change shell（add-sample + tasks.md），让 agent 按 skill 推进
  - 硬门禁：`llman sdd validate --all --strict --no-interactive`

依赖：
  - promptfoo（建议全局安装）
  - python3、git
  - llman（优先使用仓库内 `target/debug/llman`；否则使用 PATH 中的 `llman`）
  - agentdev/promptfoo/node_modules/@anthropic-ai/claude-agent-sdk（首次需安装）

常用选项：
  --skill <id>                    默认：llman-sdd-apply（要评测的 skill 模板名）
  --baseline-skill <path>         可选；上一版 SKILL.md 快照路径（baseline provider 用它渲染 prompt）
                                  默认指向 agentdev/promptfoo/baselines/ 下的人工快照
                                  不指定时 baseline == candidate（退化为单版本评测）
  --model <alias>                 默认：sonnet（Claude Code SDK 模型别名）
  --max-turns <N>                 默认：18
  --runs <N>                      默认：1（独立 run 次数，每次新 seed 根目录）
  --repeat <N>                    透传给 promptfoo eval --repeat
  --judge <off|human|codex|claude> 默认：off（可选软评分；不替代硬门禁）
  --judge-grader <provider>       可选；judge=codex/claude 时覆盖 promptfoo --grader
  --eval-retries <N>              默认：2（promptfoo eval 失败时最多重试 N 次）
  --llman-bin <path>              可选；覆盖 llman 可执行文件路径
  --api-key-env <VAR>             可选；从指定环境变量读取 API key（默认自动探测）
  --max-concurrency <N>           透传给 promptfoo eval --max-concurrency
  --delay <ms>                    透传给 promptfoo eval --delay
  --no-cache                      透传给 promptfoo eval --no-cache
  --no-run                        只生成 workspaces + promptfoo 目录，不执行 promptfoo eval
  --ui                            评测结束后启动 Promptfoo Web UI（会阻塞）
  --ui-port <N>                   Web UI 端口（默认：15500）

Claude Code account 注入（敏感）：
  --cc-account <name>             例如：glm-lite-150
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

SKILL_ID="llman-sdd-apply"
BASELINE_SKILL_PATH=""
MODEL="sonnet"
MAX_TURNS="18"
RUNS="1"
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
    --skill)
      SKILL_ID="${2:-}"; shift 2;;
    --baseline-skill)
      BASELINE_SKILL_PATH="${2:-}"; shift 2;;
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

need_cmd python3
need_cmd git

# Resolve llman binary: explicit override > repo target/debug > PATH.
if [[ -n "$LLMAN_BIN_OVERRIDE" ]]; then
  LLMAN_BIN="$LLMAN_BIN_OVERRIDE"
elif [[ -x "$REPO_ROOT/target/debug/llman" ]]; then
  LLMAN_BIN="$REPO_ROOT/target/debug/llman"
elif command -v llman >/dev/null 2>&1; then
  LLMAN_BIN="$(command -v llman)"
else
  die "找不到 llman 可执行文件：传入 --llman-bin，或先 cargo build，或确保 llman 在 PATH"
fi
echo "== llman binary: $LLMAN_BIN"

# Resolve promptfoo: prefer local install, else global.
# In --no-run mode, promptfoo is optional (dry-run only generates sandboxes + prompts).
PROMPTFOO_CMD=(promptfoo)
PROMPTFOO_AVAILABLE="1"
if ! command -v promptfoo >/dev/null 2>&1; then
  if [[ -x "$REPO_ROOT/agentdev/promptfoo/node_modules/.bin/promptfoo" ]]; then
    PROMPTFOO_CMD=("$REPO_ROOT/agentdev/promptfoo/node_modules/.bin/promptfoo")
  else
    PROMPTFOO_AVAILABLE="0"
    if [[ "$NO_RUN" != "1" ]]; then
      die "找不到 promptfoo：请全局安装或运行 pnpm -C agentdev/promptfoo install"
    fi
    echo "== promptfoo not found; --no-run will skip validate/eval"
  fi
fi

# Baseline snapshot validation.
if [[ -n "$BASELINE_SKILL_PATH" ]]; then
  [[ -f "$BASELINE_SKILL_PATH" ]] || die "--baseline-skill 指向的文件不存在：$BASELINE_SKILL_PATH"
fi

timestamp_utc="$(date -u +%Y-%m-%dT%H%M%SZ)"
git_sha="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"

# ---------------------------------------------------------------------------
# Workspace helpers (adapted from run-sdd-claude-style-eval.sh; variant replaces style)
# ---------------------------------------------------------------------------

init_workspace() {
  local variant="$1"
  local workspace_dir="$2"
  local config_dir="$3"

  mkdir -p "$workspace_dir" "$config_dir"

  echo "== init workspace ($variant): $workspace_dir"
  # skill-gate does not compare spec formats; use the repo default (toon).
  (cd "$workspace_dir" && LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd init "$workspace_dir" --lang en >/dev/null)

  mkdir -p "$workspace_dir/.llman-bin"
  cp "$LLMAN_BIN" "$workspace_dir/.llman-bin/llman"
  chmod +x "$workspace_dir/.llman-bin/llman"

  git -C "$workspace_dir" init -q
  git -C "$workspace_dir" config user.email "agentdev@example.com"
  git -C "$workspace_dir" config user.name "agentdev"
  git -C "$workspace_dir" add -A
  git -C "$workspace_dir" commit -qm "baseline"
}

seed_change_shell() {
  # Pre-seed an `add-sample` change with a small tasks.md so the apply skill
  # has something concrete to advance. Both workspaces get the same seed.
  local workspace_dir="$1"
  local config_dir="$2"

  echo "== seed change shell (apply): $workspace_dir"
  (
    cd "$workspace_dir"
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd change new add-sample >/dev/null

    # Flesh out a minimal proposal + a small tasks.md with 2 pending items.
    cat >"llmanspec/changes/add-sample/proposal.md" <<'EOF'
---
depends_on: []
---

## Why
Sample change for skill-gate evaluation: exercise the apply skill on a trivial task.

## What Changes
- Add a documentation requirement + scenario to an existing capability.
- This is intentionally minimal so the hard gate (validate --strict) is achievable.
EOF

    cat >"llmanspec/changes/add-sample/tasks.md" <<'EOF'
# Tasks: add-sample

- [ ] 1.1 Add requirement `REQ_SAMPLE_1` to capability `sdd-workflow` via `llman sdd spec add-req`
  - 验证：`llman sdd validate sdd-workflow --strict --no-interactive` 通过
- [ ] 1.2 Add a scenario for `REQ_SAMPLE_1` via `llman sdd spec add-scenario`
  - 验证：`llman sdd validate --all --strict --no-interactive` 通过
EOF

    git add -A
    git commit -qm "seed: add-sample change shell"
  )
}

seed_propose_shell() {
  # Pre-seed an empty capability spec skeleton so the propose skill has a
  # legitimate live spec to edit. No change is created (that's the agent's job).
  local workspace_dir="$1"
  local config_dir="$2"

  echo "== seed spec skeleton (propose): $workspace_dir"
  (
    cd "$workspace_dir"
    LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd spec skeleton eval-propose-cap --force >/dev/null
    git add -A
    git commit -qm "seed: eval-propose-cap skeleton" || true
  )
}

# Dispatch sandbox seeding by skill id.
# Args: skill_id  workspace_dir  config_dir
seed_for_skill() {
  local skill_id="$1"
  local workspace_dir="$2"
  local config_dir="$3"

  case "$skill_id" in
    llman-sdd-apply)
      seed_change_shell "$workspace_dir" "$config_dir"
      ;;
    llman-sdd-draft)
      # Draft needs only an empty initialized project (already done by init_workspace).
      echo "== seed (draft): no-op (empty project sufficient): $workspace_dir"
      ;;
    llman-sdd-propose)
      seed_propose_shell "$workspace_dir" "$config_dir"
      ;;
    *)
      echo "== seed (unknown skill '$skill_id'): no seeding (default to empty project)"
      ;;
  esac
}

resolve_promptfoo_anthropic_key_source_env() {
  # Mirrors run-sdd-claude-style-eval.sh: find an env var holding the API key.
  local candidates=(
    "ANTHROPIC_API_KEY"
    "ANTHROPIC_AUTH_TOKEN"
    "GLM_API_KEY"
  )
  if [[ -n "$API_KEY_ENV" ]]; then
    candidates=("$API_KEY_ENV" "${candidates[@]}")
  fi
  for v in "${candidates[@]}"; do
    local val="${!v:-}"
    if [[ -n "$val" ]]; then
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
Error: 缺少 Promptfoo/Claude Agent SDK 所需的 ANTHROPIC_API_KEY。
解决方式：
1) export ANTHROPIC_API_KEY=...
2) 或用 --api-key-env <VAR> 指定（例如 ANTHROPIC_AUTH_TOKEN / GLM_API_KEY）
EOF
    exit 1
  fi
  export ANTHROPIC_API_KEY="${!source_env}"
  [[ -n "${ANTHROPIC_API_KEY:-}" ]] || die "环境变量为空：$source_env"
  echo "== promptfoo api key source: $source_env -> ANTHROPIC_API_KEY"
}

reset_workspace_to_sha() {
  local variant="$1"
  local workspace_dir="$2"
  local sha="$3"
  echo "== reset workspace ($variant) -> $sha"
  git -C "$workspace_dir" reset --hard "$sha" >/dev/null
  git -C "$workspace_dir" clean -fdx >/dev/null
}

# ---------------------------------------------------------------------------
# SKILL.md rendering (adapted from run-sdd-prompts-eval.sh render_skill_prompt)
# ---------------------------------------------------------------------------

strip_frontmatter() {
  awk '
    NR==1 && $0=="---" { in_front=1; next }
    in_front==1 && $0=="---" { in_front=0; next }
    in_front==1 { next }
    { print }
  ' "$1"
}

# Render the skill template installed in a workspace into a text prompt file.
# Args: workspace_dir  skill_id  out_path  config_dir
render_skill_prompt() {
  local workspace_dir="$1"
  local skill_id="$2"
  local out_path="$3"
  local config_dir="$4"

  # `sdd init --update` triggers update_skills::run_with_root, which renders
  # templates into .agents/skills/<id>/SKILL.md.
  if ! (cd "$workspace_dir" && LLMAN_CONFIG_DIR="$config_dir" "$LLMAN_BIN" sdd init --update "$workspace_dir") >/dev/null 2>"$out_path.render.log"; then
    echo "Error: init --update failed in $workspace_dir. Log:" >&2
    cat "$out_path.render.log" >&2
    return 1
  fi

  local skill_path="$workspace_dir/.agents/skills/$skill_id/SKILL.md"
  [[ -f "$skill_path" ]] || die "找不到生成的 skill 产物：$skill_path（skill_id=$skill_id）"

  {
    echo "你正在执行 llman SDD workflow skill \`$skill_id\`。"
    echo "请严格遵循以下指令。"
    echo
    echo "--- BEGIN SKILL ---"
    strip_frontmatter "$skill_path"
    echo "--- END SKILL ---"
  } > "$out_path"

  # Guard: unit() must be fully expanded.
  if rg -n '{{ unit\(' "$out_path" >/dev/null 2>&1; then
    die "prompt 仍包含未展开的 unit()：$out_path"
  fi
}

# Compose the final agent_task.md: SKILL.md (system block) + task_prompt.
# Args: skill_prompt_txt  task_prompt_value  out_agent_task_md
compose_agent_task() {
  local skill_prompt_txt="$1"
  local task_prompt_value_file="$2"
  local out_agent_task_md="$3"

  {
    echo "# Task"
    echo
    echo "## Skill instructions (system)"
    echo
    cat "$skill_prompt_txt"
    echo
    echo "## Goal"
    echo
    cat "$task_prompt_value_file"
  } > "$out_agent_task_md"
}

# ---------------------------------------------------------------------------
# promptfoo fixture patching (variant replaces style)
# ---------------------------------------------------------------------------

patch_promptfoo_fixture() {
  local promptfoo_dir="$1"
  local workdir_baseline="$2"
  local workdir_candidate="$3"
  local configdir_baseline="$4"
  local configdir_candidate="$5"
  local path_baseline="$6"
  local path_candidate="$7"

  local config_path="$promptfoo_dir/promptfooconfig.yaml"
  [[ -f "$config_path" ]] || die "找不到 promptfoo config：$config_path"

  python3 - \
    "$config_path" \
    "$MODEL" \
    "$MAX_TURNS" \
    "$workdir_baseline" \
    "$workdir_candidate" \
    "$configdir_baseline" \
    "$configdir_candidate" \
    "$path_baseline" \
    "$path_candidate" \
    "$JUDGE" <<'PY'
import sys

(
    path,
    model,
    max_turns,
    workdir_baseline,
    workdir_candidate,
    configdir_baseline,
    configdir_candidate,
    path_baseline,
    path_candidate,
    judge,
) = sys.argv[1:]

max_turns = int(max_turns)

with open(path, "r", encoding="utf-8") as f:
    text = f.read()

replacements = {
    "__MODEL__": model,
    "__MAX_TURNS__": str(max_turns),
    "__WORKDIR_BASELINE__": workdir_baseline,
    "__WORKDIR_CANDIDATE__": workdir_candidate,
    "__CONFIGDIR_BASELINE__": configdir_baseline,
    "__CONFIGDIR_CANDIDATE__": configdir_candidate,
    "__PATH_BASELINE__": path_baseline,
    "__PATH_CANDIDATE__": path_candidate,
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
    text = text.replace("    " + judge_marker, "    " + block.rstrip("\n"))
else:
    text = text.replace("    " + judge_marker + "\n", "")

with open(path, "w", encoding="utf-8") as f:
    f.write(text)
PY
}

write_meta_workspace() {
  local variant="$1"
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

# ---------------------------------------------------------------------------
# Result aggregation (variant replaces style; structure mirrors styles runner)
# ---------------------------------------------------------------------------

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
lines.append("# Promptfoo Summary (skill-gate)")
lines.append(f"- evalId: `{summary.get('evalId')}`")
lines.append("")
lines.append("| variant | cases | ok | fail | err | turns(avg/max) | tokens(total) | cost(usd) | permission_denials |")
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

  python3 - "$batch_dir" "$out_json" "$out_md" "$SKILL_ID" "$MODEL" "$MAX_TURNS" "$RUNS" "$REPEAT" "$JUDGE" "$BASELINE_SKILL_PATH" <<'PY'
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
    skill_id,
    model,
    max_turns,
    runs,
    repeat,
    judge,
    baseline_skill_path,
) = sys.argv[1:]

max_turns_i = int(max_turns)
runs_i = int(runs)
repeat_i: Optional[int] = int(repeat) if repeat else None


def select_variant(provider_id: str) -> str:
    lowered = provider_id.lower()
    if "baseline" in lowered:
        return "baseline"
    if "candidate" in lowered:
        return "candidate"
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
    variant: str
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
        variant = select_variant(provider_id)

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
                variant=variant,
                success=is_success,
                error=is_error,
                tokens_total=tokens_total,
                turns=turns,
                cost_usd=cost_f,
            )
        )

variants = ["baseline", "candidate"]
variant_stats: Dict[str, Any] = {}
for variant in variants:
    vrows = [r for r in rows if r.variant == variant]
    total = len(vrows)
    ok = sum(1 for r in vrows if r.success)
    err = sum(1 for r in vrows if r.error)
    fail = total - ok - err

    tokens_vals = [float(r.tokens_total) for r in vrows if r.tokens_total is not None]
    turns_vals = [float(r.turns) for r in vrows if r.turns is not None]
    cost_vals = [float(r.cost_usd) for r in vrows if r.cost_usd is not None]

    variant_stats[variant] = {
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
        "skill_id": skill_id,
        "model": model,
        "max_turns": max_turns_i,
        "runs": runs_i,
        "repeat": repeat_i,
        "judge": judge,
        "baseline_skill_path": baseline_skill_path or None,
    },
    "variants": variant_stats,
}

with open(out_json_path, "w", encoding="utf-8") as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)

lines = []
lines.append("# Skill-gate Batch Aggregate")
lines.append("")
lines.append(f"- skill_id: `{skill_id}`")
lines.append(f"- model: `{model}`, max_turns: {max_turns_i}, runs: {runs_i}, repeat: {repeat_i}")
lines.append(f"- baseline_skill_path: `{baseline_skill_path or '(same as candidate)'}`")
lines.append("")
lines.append("| variant | cases | ok | fail | err | pass_rate | tokens(mean/median/p90) | turns(mean/median/p90) | cost(mean) |")
lines.append("|---|---:|---:|---:|---:|---:|---|---|---:|")
for variant in variants:
    vals = variant_stats[variant]
    pr = vals["pass_rate"]
    pr_s = f"{pr:.1%}" if isinstance(pr, (int, float)) else "-"
    tk = vals["tokens_total"]
    tn = vals["turns"]
    co = vals["cost_usd"]
    def fmt_stats(s):
        if s["n"] == 0:
            return "-/-/-"
        mean = f"{s['mean']:.0f}" if s["mean"] is not None else "-"
        median = f"{s['median']:.0f}" if s["median"] is not None else "-"
        p90 = f"{s['p90']:.0f}" if s["p90"] is not None else "-"
        return f"{mean}/{median}/{p90}"
    cost_mean = f"{co['mean']:.6f}" if co["mean"] is not None else "-"
    lines.append(
        f"| {variant} | {vals['cases']} | {vals['successes']} | {vals['failures']} | {vals['errors']} "
        f"| {pr_s} | {fmt_stats(tk)} | {fmt_stats(tn)} | {cost_mean} |"
    )

with open(out_md_path, "w", encoding="utf-8") as f:
    f.write("\n".join(lines) + "\n")
PY
}

# ---------------------------------------------------------------------------
# Per-run orchestration
# ---------------------------------------------------------------------------

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
    source <("$LLMAN_BIN" --config-dir "$CC_CONFIG_DIR" x claude-code account env "$CC_ACCOUNT")
  fi
  if [[ "$NO_RUN" != "1" ]]; then
    ensure_promptfoo_anthropic_api_key
  fi

  local ws_baseline="$workspaces_dir/baseline"
  local ws_candidate="$workspaces_dir/candidate"
  local cfg_baseline="$configs_dir/baseline"
  local cfg_candidate="$configs_dir/candidate"

  init_workspace "baseline" "$ws_baseline" "$cfg_baseline"
  init_workspace "candidate" "$ws_candidate" "$cfg_candidate"

  seed_for_skill "$SKILL_ID" "$ws_baseline" "$cfg_baseline"
  seed_for_skill "$SKILL_ID" "$ws_candidate" "$cfg_candidate"

  local baseline_sha_baseline
  local baseline_sha_candidate
  baseline_sha_baseline="$(git -C "$ws_baseline" rev-parse HEAD)"
  baseline_sha_candidate="$(git -C "$ws_candidate" rev-parse HEAD)"

  local path_baseline="$ws_baseline/.llman-bin:$PATH"
  local path_candidate="$ws_candidate/.llman-bin:$PATH"

  export SDD_WORKDIR_BASELINE="$ws_baseline"
  export SDD_WORKDIR_CANDIDATE="$ws_candidate"
  export SDD_CONFIGDIR_BASELINE="$cfg_baseline"
  export SDD_CONFIGDIR_CANDIDATE="$cfg_candidate"

  # --- Render SKILL.md into prompts --------------------------------------
  echo
  echo "== render SKILL.md (candidate = current workspace template)"
  local skill_prompt_candidate="$meta_dir/skill-candidate.txt"
  render_skill_prompt "$ws_candidate" "$SKILL_ID" "$skill_prompt_candidate" "$cfg_candidate"

  local skill_prompt_baseline="$meta_dir/skill-baseline.txt"
  if [[ -n "$BASELINE_SKILL_PATH" ]]; then
    # Render candidate first (to populate .codex/skills), then overwrite with snapshot.
    render_skill_prompt "$ws_baseline" "$SKILL_ID" "$skill_prompt_baseline" "$cfg_baseline"
    echo "== overwrite baseline skill with snapshot: $BASELINE_SKILL_PATH"
    {
      echo "你正在执行 llman SDD workflow skill \`$SKILL_ID\`。"
      echo "请严格遵循以下指令。"
      echo
      echo "--- BEGIN SKILL ---"
      strip_frontmatter "$BASELINE_SKILL_PATH"
      echo "--- END SKILL ---"
    } > "$skill_prompt_baseline"
  else
    # No snapshot: baseline == candidate (degenerate single-version eval).
    echo "== no --baseline-skill: baseline uses same template as candidate"
    render_skill_prompt "$ws_baseline" "$SKILL_ID" "$skill_prompt_baseline" "$cfg_baseline"
  fi

  # --- Compose agent_task.md per variant ---------------------------------
  echo
  echo "== prepare promptfoo fixture"
  local fixture_src="$REPO_ROOT/agentdev/promptfoo/sdd_skill_gate_v1"
  [[ -d "$fixture_src" ]] || die "找不到 promptfoo fixture：$fixture_src"
  cp -R "$fixture_src/." "$promptfoo_dir/"

  if [[ ! -e "$promptfoo_dir/node_modules" ]]; then
    ln -s "$REPO_ROOT/agentdev/promptfoo/node_modules" "$promptfoo_dir/node_modules"
  fi

  # Extract task_prompt from tests.yaml matching the target skill into a temp file.
  local task_prompt_file="$meta_dir/task_prompt.txt"
  python3 - "$promptfoo_dir/tests.yaml" "$task_prompt_file" "$SKILL_ID" <<'PY'
import sys
import yaml
path, out, skill = sys.argv[1], sys.argv[2], sys.argv[3]
with open(path, "r", encoding="utf-8") as f:
    tests = yaml.safe_load(f)
task = ""
if isinstance(tests, list):
    # Prefer the test case whose vars.skill matches; fall back to the first case.
    matched = None
    for t in tests:
        vars_ = (t.get("vars") if isinstance(t, dict) else None) or {}
        if vars_.get("skill") == skill:
            matched = t
            break
    if matched is None and tests:
        matched = tests[0]
    if matched is not None:
        vars_ = matched.get("vars") or {}
        task = vars_.get("task_prompt") or ""
with open(out, "w", encoding="utf-8") as f:
    f.write(task)
PY

  # NOTE: both variants share the same promptfoo_dir, but promptfoo drives
  # each provider with the SAME prompts/agent_task.md. For skill-gate we need
  # per-variant SKILL.md. Since promptfoo does not support per-provider prompt
  # P3: per-provider prompt override. Generate two distinct prompt files so the
  # baseline provider sees the baseline-skill text and the candidate provider
  # sees the candidate-skill text — a true A/B in one eval run. The shared
  # agent_task.md is kept as a fallback (promptfoo requires a top-level prompts
  # entry even when all providers override it); we point it at the candidate.
  local composed_baseline="$promptfoo_dir/prompts/agent_task_baseline.md"
  local composed_candidate="$promptfoo_dir/prompts/agent_task_candidate.md"
  compose_agent_task "$skill_prompt_baseline" "$task_prompt_file" "$composed_baseline"
  compose_agent_task "$skill_prompt_candidate" "$task_prompt_file" "$composed_candidate"
  # Shared fallback (identical to candidate).
  cp "$composed_candidate" "$promptfoo_dir/prompts/agent_task.md"

  patch_promptfoo_fixture "$promptfoo_dir" "$ws_baseline" "$ws_candidate" "$cfg_baseline" "$cfg_candidate" "$path_baseline" "$path_candidate"

  echo
  if [[ "$PROMPTFOO_AVAILABLE" == "1" ]]; then
    echo "== promptfoo validate config"
    (cd "$promptfoo_dir" && "${PROMPTFOO_CMD[@]}" validate config -c "$promptfoo_dir/promptfooconfig.yaml")
  else
    echo "== (skipping promptfoo validate config: promptfoo not available in --no-run)"
  fi

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
        reset_workspace_to_sha "baseline" "$ws_baseline" "$baseline_sha_baseline"
        reset_workspace_to_sha "candidate" "$ws_candidate" "$baseline_sha_candidate"
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
  write_meta_workspace "baseline" "$ws_baseline" "$cfg_baseline" "$meta_dir/baseline"
  write_meta_workspace "candidate" "$ws_candidate" "$cfg_candidate" "$meta_dir/candidate"

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

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if (( RUNS < 1 )); then
  die "--runs 必须 >= 1"
fi

batch_seed="$(python3 - <<'PY'
import os
print(os.urandom(4).hex())
PY
)"
BATCH_DIR="$REPO_ROOT/.tmp/sdd-skill-gate-eval/${timestamp_utc}_${git_sha}_${SKILL_ID}_b${batch_seed}"
mkdir -p "$BATCH_DIR/meta" "$BATCH_DIR/runs"
echo "== batch_dir: $BATCH_DIR"
LAST_BATCH_DIR="$BATCH_DIR"

echo "== skill_id:   $SKILL_ID"
echo "== model:      $MODEL"
echo "== baseline:   ${BASELINE_SKILL_PATH:-(same as candidate)}"

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
