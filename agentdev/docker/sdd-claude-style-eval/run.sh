#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

usage() {
  cat <<'EOF'
用法：
  bash agentdev/docker/sdd-claude-style-eval/run.sh [docker options] -- [eval options]

说明：
  - 构建并运行 docker 镜像，执行 `scripts/sdd-claude-style-eval.sh`
  - 默认将宿主机输出目录挂载到容器的 `/repo/.tmp`，以持久化 workspaces + results

Docker options:
  --tag <name>                 镜像 tag（默认：llman/sdd-claude-style-eval:local）
  --out <dir>                  输出目录（默认：./.tmp/docker-sdd-claude-style-eval/<ts>）
  --apt-mirror-debian <url>    APT Debian 镜像（例如 http://mirrors.aliyun.com/debian）
  --apt-mirror-security <url>  APT Security 镜像（例如 http://mirrors.aliyun.com/debian-security）
  --npm-registry <url>         npm registry（例如 https://registry.npmmirror.com）
  --pip-index-url <url>        PyPI index（例如 https://mirrors.aliyun.com/pypi/simple）
  --pip-trusted-host <host>    PyPI trusted host（可选）
  --promptfoo-version <ver>    promptfoo 版本（默认：0.121.2）

Eval options（传给容器内脚本）示例：
  -- --model sonnet --max-turns 18 --runs 1

环境变量透传：
  - ANTHROPIC_API_KEY（必须：用于 Claude Code agentic）
  - OPENAI_API_KEY（可选：judge=codex 时用于 rubric）
EOF
}

die() {
  echo "Error: $*" >&2
  exit 1
}

TAG="llman/sdd-claude-style-eval:local"
OUT_DIR=""

APT_MIRROR_DEBIAN=""
APT_MIRROR_SECURITY=""
NPM_REGISTRY=""
PIP_INDEX_URL=""
PIP_TRUSTED_HOST=""
PROMPTFOO_VERSION=""

EVAL_ARGS=()
SEEN_DASH_DASH="0"

while [[ $# -gt 0 ]]; do
  if [[ "$SEEN_DASH_DASH" == "1" ]]; then
    EVAL_ARGS+=("$1"); shift 1; continue
  fi
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --)
      SEEN_DASH_DASH="1"; shift 1;;
    --tag)
      TAG="${2:-}"; shift 2;;
    --out)
      OUT_DIR="${2:-}"; shift 2;;
    --apt-mirror-debian)
      APT_MIRROR_DEBIAN="${2:-}"; shift 2;;
    --apt-mirror-security)
      APT_MIRROR_SECURITY="${2:-}"; shift 2;;
    --npm-registry)
      NPM_REGISTRY="${2:-}"; shift 2;;
    --pip-index-url)
      PIP_INDEX_URL="${2:-}"; shift 2;;
    --pip-trusted-host)
      PIP_TRUSTED_HOST="${2:-}"; shift 2;;
    --promptfoo-version)
      PROMPTFOO_VERSION="${2:-}"; shift 2;;
    *)
      die "未知参数：$1（使用 --help 查看）"
      ;;
  esac
done

timestamp_utc="$(date -u +%Y-%m-%dT%H%M%SZ)"
if [[ -z "$OUT_DIR" ]]; then
  OUT_DIR="$REPO_ROOT/.tmp/docker-sdd-claude-style-eval/${timestamp_utc}"
fi
mkdir -p "$OUT_DIR"

build_args=()
if [[ -n "$APT_MIRROR_DEBIAN" ]]; then build_args+=(--build-arg "APT_MIRROR_DEBIAN=$APT_MIRROR_DEBIAN"); fi
if [[ -n "$APT_MIRROR_SECURITY" ]]; then build_args+=(--build-arg "APT_MIRROR_SECURITY=$APT_MIRROR_SECURITY"); fi
if [[ -n "$NPM_REGISTRY" ]]; then build_args+=(--build-arg "NPM_REGISTRY=$NPM_REGISTRY"); fi
if [[ -n "$PIP_INDEX_URL" ]]; then build_args+=(--build-arg "PIP_INDEX_URL=$PIP_INDEX_URL"); fi
if [[ -n "$PIP_TRUSTED_HOST" ]]; then build_args+=(--build-arg "PIP_TRUSTED_HOST=$PIP_TRUSTED_HOST"); fi
if [[ -n "$PROMPTFOO_VERSION" ]]; then build_args+=(--build-arg "PROMPTFOO_VERSION=$PROMPTFOO_VERSION"); fi

echo "== docker build: $TAG"
docker build \
  -f "$REPO_ROOT/agentdev/docker/sdd-claude-style-eval/Dockerfile" \
  -t "$TAG" \
  "${build_args[@]}" \
  "$REPO_ROOT"

echo
echo "== docker run"
echo "out_dir: $OUT_DIR"

env_args=()
if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then env_args+=(-e "ANTHROPIC_API_KEY"); fi
if [[ -n "${OPENAI_API_KEY:-}" ]]; then env_args+=(-e "OPENAI_API_KEY"); fi

docker run --rm -it \
  "${env_args[@]}" \
  -v "$OUT_DIR:/repo/.tmp" \
  "$TAG" \
  bash scripts/sdd-claude-style-eval.sh "${EVAL_ARGS[@]}"
