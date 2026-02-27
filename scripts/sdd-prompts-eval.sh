#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'EOF'
用法：
  bash scripts/sdd-prompts-eval.sh [options]

说明：
  - 在临时目录创建一个最小 llmanspec 项目（不会污染仓库根目录）
  - 基于当前工作区模板渲染生成 baseline/candidate 的 Arena system prompt
  - 复制 arena contest/dataset fixtures 到临时 LLMAN_CONFIG_DIR 并运行 `llman x arena gen`

必要环境变量：
  - OPENAI_API_KEY
  - OPENAI_DEFAULT_MODEL（推荐；也可用 --model 覆盖）
  - OPENAI_BASE_URL 或 OPENAI_API_BASE（可选；用于中转/加速；建议带 /v1）

常用选项：
  --locale <en|zh-Hans>          默认：zh-Hans
  --baseline-style <new|legacy>  默认：legacy
  --candidate-style <new|legacy> 默认：new
  --model <model_id>             默认：$OPENAI_DEFAULT_MODEL（若两者都为空则报错）
  --rounds <N>                   默认：10
  --seed <S>                     可选；不传则使用随机 seed
  --no-gen                       只生成 prompts/fixtures，不跑 arena gen
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

LOCALE="zh-Hans"
BASELINE_STYLE="legacy"
CANDIDATE_STYLE="new"
MODEL="${OPENAI_DEFAULT_MODEL:-}"
ROUNDS="10"
SEED=""
NO_GEN="0"

CONTEST_NAME="sdd_apply_v1"
DATASET_NAME="sdd_apply_v1"
PROMPT_BASELINE="sdd_apply_v1_baseline"
PROMPT_CANDIDATE="sdd_apply_v1_candidate"
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
    --model)
      MODEL="${2:-}"; shift 2;;
    --rounds)
      ROUNDS="${2:-}"; shift 2;;
    --seed)
      SEED="${2:-}"; shift 2;;
    --no-gen)
      NO_GEN="1"; shift 1;;
    *)
      die "未知参数：$1（使用 --help 查看）"
      ;;
  esac
done

[[ -n "${MODEL}" ]] || die "必须指定模型：设置 OPENAI_DEFAULT_MODEL 或传入 --model"

timestamp_utc="$(date -u +%Y-%m-%dT%H%M%SZ)"
git_sha="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
work_dir="$REPO_ROOT/.tmp/sdd-prompts-eval/${timestamp_utc}_${git_sha}"
project_dir="$work_dir/project_${LOCALE}"
config_dir="$work_dir/config"
meta_dir="$work_dir/meta"

mkdir -p "$project_dir" "$config_dir" "$meta_dir"

echo "== work_dir:  $work_dir"
echo "== locale:    $LOCALE"
echo "== model:     $MODEL"
echo "== baseline:  $BASELINE_STYLE"
echo "== candidate: $CANDIDATE_STYLE"
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
echo "== sdd validate --ab-report (locale-scoped)"
run_llman "$project_dir" sdd validate --ab-report --json --no-interactive \
  2> "$meta_dir/sdd-ab-report.stderr" | tee "$meta_dir/sdd-ab-report.json" >/dev/null

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

  run_llman "$project_dir" sdd update-skills \
    --tool codex \
    --no-interactive \
    --skills-only \
    --style "$style" \
    2>&1 | tee "$meta_dir/update-skills-${style}.txt" >/dev/null

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
echo "== generate baseline/candidate prompts"
mkdir -p "$config_dir/prompt/codex"
render_skill_prompt "$BASELINE_STYLE" "$config_dir/prompt/codex/${PROMPT_BASELINE}.md"
render_skill_prompt "$CANDIDATE_STYLE" "$config_dir/prompt/codex/${PROMPT_CANDIDATE}.md"

echo
echo "== install arena fixtures into temp config"
mkdir -p "$config_dir/arena/contests" "$config_dir/arena/datasets"
cp "$REPO_ROOT/artifacts/testing_config_home/arena/contests/${CONTEST_NAME}.toml" "$config_dir/arena/contests/"
cp "$REPO_ROOT/artifacts/testing_config_home/arena/datasets/${DATASET_NAME}.yaml" "$config_dir/arena/datasets/"

echo
echo "== pin contest model"
perl -pi -e "s/^models\\s*=\\s*\\[[^\\]]*\\]/models = [\\\"$MODEL\\\"]/g" \
  "$config_dir/arena/contests/${CONTEST_NAME}.toml"

echo
echo "== ready"
echo "LLMAN_CONFIG_DIR=$config_dir"
echo "Prompts:"
echo "  - prompt/codex/${PROMPT_BASELINE}.md"
echo "  - prompt/codex/${PROMPT_CANDIDATE}.md"
echo "Contest/Dataset:"
echo "  - arena/contests/${CONTEST_NAME}.toml"
echo "  - arena/datasets/${DATASET_NAME}.yaml"

if [[ "$NO_GEN" == "1" ]]; then
  echo
  echo "（跳过 gen：因为传入了 --no-gen）"
  exit 0
fi

[[ -n "${OPENAI_API_KEY:-}" ]] || die "OPENAI_API_KEY 未设置（Arena gen 需要）"

echo
echo "== arena gen"
gen_args=(x arena gen --contest "$CONTEST_NAME" --dataset "$DATASET_NAME" --rounds "$ROUNDS")
if [[ -n "$SEED" ]]; then
  gen_args+=(--seed "$SEED")
fi

(cd "$REPO_ROOT" && LLMAN_CONFIG_DIR="$config_dir" cargo run -q -- "${gen_args[@]}")

cat <<EOF

下一步（在同一 LLMAN_CONFIG_DIR 下）：
  LLMAN_CONFIG_DIR=$config_dir cargo run -q -- x arena vote --run <RUN_ID>
  LLMAN_CONFIG_DIR=$config_dir cargo run -q -- x arena report --run <RUN_ID>
EOF
