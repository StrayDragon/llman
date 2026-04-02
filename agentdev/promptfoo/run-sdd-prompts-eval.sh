#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat <<'EOF'
用法：
  bash agentdev/promptfoo/run-sdd-prompts-eval.sh [options]

兼容入口：
  bash scripts/sdd-prompts-eval.sh [options]

说明：
  - 在临时目录创建一个最小 llmanspec 项目（不会污染仓库根目录）
  - 基于当前工作区模板渲染生成 baseline/candidate 的 Promptfoo system prompt
  - 复制 promptfoo fixtures 到临时目录并运行 `promptfoo validate/eval`

必要环境变量：
  - OPENAI_API_KEY
  - OPENAI_BASE_URL 或 OPENAI_API_BASE（可选；用于中转/加速；建议带 /v1）

常用选项：
  --locale <en|zh-Hans>          默认：zh-Hans
  --baseline-style <new>         默认：new（legacy 已移除）
  --candidate-style <new>        默认：new（legacy 已移除）
  --models <csv>                 默认：从 agentdev/promptfoo/default_models.txt 读取
  --repeat <N>                   可选；等价于 `promptfoo eval --repeat`
  --max-concurrency <N>          可选；等价于 `promptfoo eval --max-concurrency`
  --delay <ms>                   可选；等价于 `promptfoo eval --delay`
  --no-cache                     可选；等价于 `promptfoo eval --no-cache`
  --no-gen                       只生成 prompts/fixtures，不跑 promptfoo eval
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

LOCALE="zh-Hans"
BASELINE_STYLE="new"
CANDIDATE_STYLE="new"
MODELS_CSV=""
REPEAT=""
MAX_CONCURRENCY=""
DELAY_MS=""
NO_CACHE="0"
NO_GEN="0"

PROMPTFOO_FIXTURE="sdd_apply_v1"
SKILL_ID="llman-sdd-apply"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --locale)
      LOCALE="${2:-}"; shift 2;;
    --baseline-style)
      BASELINE_STYLE="${2:-}"; shift 2;;
    --candidate-style)
      CANDIDATE_STYLE="${2:-}"; shift 2;;
    --models)
      MODELS_CSV="${2:-}"; shift 2;;
    --repeat)
      REPEAT="${2:-}"; shift 2;;
    --max-concurrency)
      MAX_CONCURRENCY="${2:-}"; shift 2;;
    --delay)
      DELAY_MS="${2:-}"; shift 2;;
    --no-cache)
      NO_CACHE="1"; shift 1;;
    --no-gen)
      NO_GEN="1"; shift 1;;
    *)
      die "未知参数：$1（使用 --help 查看）"
      ;;
  esac
done

default_models_file="$REPO_ROOT/agentdev/promptfoo/default_models.txt"

load_models_from_file() {
  local path="$1"
  [[ -f "$path" ]] || die "找不到默认模型列表：$path"
  rg -v '^\s*(#|$)' "$path" || true
}

parse_models_csv() {
  local csv="$1"
  echo "$csv" | tr ',' '\n' | sed 's/^\s*//; s/\s*$//' | rg -v '^\s*$'
}

models=()
if [[ -n "$MODELS_CSV" ]]; then
  while IFS= read -r line; do models+=("$line"); done < <(parse_models_csv "$MODELS_CSV")
else
  while IFS= read -r line; do models+=("$line"); done < <(load_models_from_file "$default_models_file" | sed 's/^\s*//; s/\s*$//' | rg -v '^\s*$')
fi

(( ${#models[@]} > 0 )) || die "必须指定模型：传入 --models 或在 $default_models_file 填写默认模型列表"

timestamp_utc="$(date -u +%Y-%m-%dT%H%M%SZ)"
git_sha="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
work_dir="$REPO_ROOT/.tmp/sdd-prompts-eval/${timestamp_utc}_${git_sha}"
project_dir="$work_dir/project_${LOCALE}"
config_dir="$work_dir/config"
meta_dir="$work_dir/meta"
promptfoo_dir="$work_dir/promptfoo"

mkdir -p "$project_dir" "$config_dir" "$meta_dir" "$promptfoo_dir"

echo "== work_dir:  $work_dir"
echo "== locale:    $LOCALE"
echo "== models:    $(IFS=,; echo "${models[*]}")"
echo "== baseline:  $BASELINE_STYLE"
echo "== candidate: $CANDIDATE_STYLE"
echo "== promptfoo: $promptfoo_dir"
echo

echo "== check-sdd-templates"
python3 "$REPO_ROOT/scripts/check-sdd-templates.py" | tee "$meta_dir/check-sdd-templates.txt"

run_llman() {
  (cd "$1" && LLMAN_CONFIG_DIR="$config_dir" cargo run -q --manifest-path "$REPO_ROOT/Cargo.toml" -- "${@:2}")
}

echo
echo "== init temp llmanspec project"
run_llman "$REPO_ROOT" sdd init "$project_dir" --lang "$LOCALE" 2>&1 | tee "$meta_dir/sdd-init.txt"

echo
echo "== sdd validate (locale-scoped)"
run_llman "$project_dir" sdd validate --all --strict --json --no-interactive \
  2> "$meta_dir/sdd-validate.stderr" | tee "$meta_dir/sdd-validate.json" >/dev/null

strip_frontmatter() {
  awk '
    NR==1 && $0=="---" { in_front=1; next }
    in_front==1 && $0=="---" { in_front=0; next }
    in_front==1 { next }
    { print }
  ' "$1"
}

render_skill_prompt() {
  local style="$1"
  local out_path="$2"
  local skill_path="$project_dir/.codex/skills/$SKILL_ID/SKILL.md"

  local cmd="sdd"
  if [[ "$style" == "legacy" ]]; then
    die "legacy style 已移除：请使用 new"
  elif [[ "$style" == "new" ]]; then
    cmd="sdd"
  else
    die "未知 style：$style（应为 new）"
  fi

  if ! run_llman "$project_dir" "$cmd" update-skills \
    --tool codex \
    --no-interactive \
    --skills-only \
    2>&1 | tee "$meta_dir/update-skills-${style}.txt" >/dev/null; then
    echo "Error: $cmd update-skills failed. Log:" >&2
    cat "$meta_dir/update-skills-${style}.txt" >&2
    exit 1
  fi

  [[ -f "$skill_path" ]] || die "找不到生成产物：$skill_path"

  {
    echo "你正在执行 llman SDD workflow skill \`$SKILL_ID\`。"
    echo "请严格遵循以下指令。"
    echo
    echo "--- BEGIN SKILL ---"
    strip_frontmatter "$skill_path"
    echo "--- END SKILL ---"
  } > "$out_path"

  if rg -n "{{ unit\\(" "$out_path" >/dev/null 2>&1; then
    die "prompt 仍包含未展开的 unit()：$out_path"
  fi
}

echo
echo "== prepare promptfoo fixtures"
fixture_src="$REPO_ROOT/agentdev/promptfoo/$PROMPTFOO_FIXTURE"
[[ -d "$fixture_src" ]] || die "找不到 promptfoo fixtures：$fixture_src"
cp -R "$fixture_src/." "$promptfoo_dir/"

echo
echo "== pin promptfoo providers"
python3 - "$promptfoo_dir/promptfooconfig.yaml" "${models[@]}" <<'PY'
import sys

path = sys.argv[1]
models = sys.argv[2:]

if not models:
    raise SystemExit("No models provided")

with open(path, "r", encoding="utf-8") as f:
    text = f.read()

needle = "  - __PROVIDERS__"
if needle not in text:
    raise SystemExit(f"Promptfoo fixture placeholder not found: {needle} in {path}")

providers_block = "\n".join([f"  - openai:chat:{m}" for m in models])
text = text.replace(needle, providers_block)

with open(path, "w", encoding="utf-8") as f:
    f.write(text)
PY

echo
echo "== generate baseline/candidate system prompts"
baseline_txt="$meta_dir/system-baseline.txt"
candidate_txt="$meta_dir/system-candidate.txt"
render_skill_prompt "$BASELINE_STYLE" "$baseline_txt"
render_skill_prompt "$CANDIDATE_STYLE" "$candidate_txt"

write_promptfoo_chat_prompt() {
  local system_prompt_txt="$1"
  local out_path="$2"

  mkdir -p "$(dirname "$out_path")"
  {
    echo "- role: system"
    echo "  content: |"
    sed 's/^/    /' "$system_prompt_txt"
    echo "- role: user"
    echo "  content: |"
    echo "    {{ task_prompt }}"
    echo
  } > "$out_path"
}

echo
echo "== write promptfoo prompt templates"
write_promptfoo_chat_prompt "$baseline_txt" "$promptfoo_dir/prompts/baseline.yaml"
write_promptfoo_chat_prompt "$candidate_txt" "$promptfoo_dir/prompts/candidate.yaml"

echo
echo "== ready"
echo "LLMAN_CONFIG_DIR=$config_dir"
echo "Promptfoo:"
echo "  - $promptfoo_dir/promptfooconfig.yaml"
echo "  - $promptfoo_dir/tests.yaml"
echo "  - $promptfoo_dir/prompts/baseline.yaml"
echo "  - $promptfoo_dir/prompts/candidate.yaml"

if [[ "$NO_GEN" == "1" ]]; then
  echo
  echo "（跳过 gen：因为传入了 --no-gen）"
  exit 0
fi

[[ -n "${OPENAI_API_KEY:-}" ]] || die "OPENAI_API_KEY 未设置（Promptfoo eval 需要）"

echo
echo "== promptfoo validate"
(cd "$promptfoo_dir" && promptfoo validate --config "$promptfoo_dir/promptfooconfig.yaml")

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
(cd "$promptfoo_dir" && "${eval_args[@]}")

cat <<EOF

下一步：
  cd $promptfoo_dir && promptfoo view --config $promptfoo_dir/promptfooconfig.yaml
EOF
