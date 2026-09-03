default:
    @just --list

# =============================================================================
# 构建和运行命令
# =============================================================================

# 构建项目
build:
    cargo build

# 构建发布版本
build-release:
    cargo build --release

# 运行项目（使用测试配置）
run *args:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- {{args}}

# 使用生产配置运行
run-prod *args:
    cargo run -- {{args}}

# 安装到本地
install:
    cargo install --path .

# =============================================================================
# 发布命令
# =============================================================================

# git-tag 分发：打带注释 tag 并推送（配合
# `cargo install --git https://github.com/StrayDragon/llman --tag v<version>`）。
# 不做 crates.io 发布：workspace 的 git 依赖（gherkin fork）会被发布归一化剥成
# registry 版本，编译期嵌入（locales/templates）也会在独立 .crate 里失去路径。
# 版本号取自 workspace.package.version；tag 已存在或工作区脏时会拒绝。
release:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! git diff --quiet || ! git diff --cached --quiet; then
        echo "❌ working tree dirty — commit first"
        exit 1
    fi
    VERSION="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -1)"
    TAG="v$VERSION"
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        echo "❌ tag $TAG already exists — bump workspace.package.version first"
        exit 1
    fi
    git tag -a "$TAG" -m "release $TAG"
    # push main first so the tag's commit is reachable, then the tag itself
    git push origin main "$TAG"
    echo "✅ $TAG pushed — install with:"
    echo "   cargo install --git https://github.com/StrayDragon/llman --tag $TAG"

# 清理构建产物
clean:
    cargo clean

# 清理 validate full mode 残留的 BDD 校验沙箱 target 目录（用后不回收会持续累积，
# why 记录见 llmanspec/changes/src-cleanup-pre-split/proposal.md「磁盘卫生」）
clean-bdd-targets:
    #!/usr/bin/env bash
    set -euo pipefail
    if compgen -G "target/bdd-*" >/dev/null; then
        du -sh target/bdd-* || true
        rm -rf target/bdd-*
        echo "✅ cleaned target/bdd-*"
    else
        echo "no target/bdd-* dirs to clean"
    fi

# =============================================================================
# 测试命令
# =============================================================================

# 运行测试（优先 cargo-nextest 并发；未安装则回退 cargo test）
# T11 拆出 crates/llman-core 后根包默认只测根包自身，必须显式 --workspace
test:
    if command -v cargo-nextest >/dev/null; then cargo nextest run --workspace --profile ci; else cargo test --workspace; fi

# 运行 BDD 测试（feature-as-spec 可执行验证，需 --features bdd）
test-bdd:
    cargo test --features bdd

# =============================================================================
# 代码质量检查
# =============================================================================

# 代码格式化
fmt:
    cargo fmt

# 检查代码格式化（不修改文件）
fmt-check:
    cargo fmt --all -- --check

# 代码检查（clippy，包含重要警告）
lint:
    cargo clippy -- -D warnings

# 快速编译检查
check-compile:
    cargo check --all-targets

# 文档检查（rustdoc warnings 视为错误）
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --document-private-items

# 核心检查（格式化检查 + lint + 测试）
check: fmt-check lint test

# 完整检查（核心检查 + 文档 + release构建 + SDD模板检查）
check-all: check doc-check build-release check-sdd-templates check-schemas

# 本地质量审计：完整检查 + i18n 键审计 + 未用依赖扫描
qa: check-all check-i18n machete

# =============================================================================
# 工具命令
# =============================================================================

# 创建新的规则模板
create-dev-template name content:
    @echo "{{content}}" > ./artifacts/testing_config_home/prompt/cursor/{{name}}.mdc
    @echo "✅ 模板 {{name}} 已创建"

# i18n 键审计（死键 / 缺失键；--fix 自动摘除死键块）
check-i18n *args:
    ./scripts/check-i18n-keys.py {{args}}

# 扫描未在代码中使用的 crate 依赖（未安装则跳过并提示）
machete:
    @command -v cargo-machete >/dev/null 2>&1 && cargo machete || echo "skip: cargo-machete 未安装（cargo install cargo-machete --locked）"

# 检查 SDD 模板版本与本地化一致性
check-sdd-templates:
    ./scripts/check-sdd-templates.py

# 评估 SDD prompts（临时目录：生成 baseline/candidate prompts + promptfoo eval）
sdd-prompts-eval *args:
    bash ./scripts/sdd-prompts-eval.sh {{args}}

# Claude Code agentic multi-style eval（ison/toon/yaml；硬门禁：sdd validate --strict；支持 --fixture v1|v2；--runs N>=2 生成 aggregate）
sdd-claude-style-eval *args:
    bash ./scripts/sdd-claude-style-eval.sh {{args}}

# 重新生成并检查配置 schema
check-schemas:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- self schema generate
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- self schema check
