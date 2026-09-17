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

# 发布到 crates.io（依赖顺序：llman-core → llman）。
# 可追加额外参数（如 `just publish --dry-run`）。
# 版本 SSOT 在 workspace.package.version，已发布的版本号需先 bump。
publish *args:
    #!/usr/bin/env bash
    set -euo pipefail
    for crate in llman-core llman; do
        ver="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print([p['version'] for p in d['packages'] if p['name']=='$crate'][0])")"
        if curl -sf -H "User-Agent: llman-publish" "https://crates.io/api/v1/crates/$crate/$ver" >/dev/null 2>&1; then
            echo "skip $crate@$ver (already on crates.io)"
        else
            echo "publish $crate@$ver ..."
            cargo publish -p "$crate" "$@"
        fi
    done

# git-tag 分发：打带注释 tag 并推送（配合
# `cargo install --git https://github.com/StrayDragon/llman --tag v<version>`）。
# crates.io 发行走 `just publish`；本 recipe 用于不走 registry 的 tag 安装源。
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

# =============================================================================
# 测试命令
# =============================================================================

# 运行测试（优先 cargo-nextest 并发；未安装则回退 cargo test）
# T11 拆出 crates/llman-core 后根包默认只测根包自身，必须显式 --workspace
# CI 与本地一致：nextest 不跑 doctest，所以 nextest 分支后补 `cargo test --doc
# --workspace`（CI 无 nextest 时 fallback 的 cargo test 已含 doctest，不重复）。
# 静默输出（省 token）：status/final 都只报 fail——成功时仅剩一行 Summary，
# 失败时失败详情完整可见；--cargo-quiet 压掉 Compiling 噪音（错误照常输出）。
# 退出码语义不变，CI 用法不受影响。
test:
    if command -v cargo-nextest >/dev/null; then cargo nextest run --workspace --profile ci --cargo-quiet --status-level fail --final-status-level fail && cargo test --doc --workspace -q; else cargo test --workspace -q; fi

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

# 完整检查（核心检查 + 文档 + release构建 + README 托管段一致性）
check-all: check doc-check build-release check-schemas check-readme

# 本地质量审计：完整检查 + i18n 键审计 + 未用依赖扫描 + 供应链审计
#（覆盖 CI 全部 job：Test Suite=check-all、Build Check=build-release、
#  Supply-chain=deny；另有 check-i18n/machete 富余）
qa: check-all check-i18n machete deny

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

# 供应链审计：许可 allowlist + RustSec 漏洞 + ban 策略（deny.toml），对齐 CI 的
# Supply-chain job（EmbarkStudios/cargo-deny-action）（未安装则跳过并提示）
deny:
    @command -v cargo-deny >/dev/null 2>&1 && cargo deny check || echo "skip: cargo-deny 未安装（cargo install cargo-deny --locked）"

# 重新生成 README 的托管段（README:GENERATED 标记：安装版本号 + 命令一览）
readme:
    ./scripts/gen_readme.py

# 校验 README 托段是否最新（过期即非零退出；已接入 check-all / CI）
check-readme:
    ./scripts/gen_readme.py --check

# 重新生成并检查配置 schema
check-schemas:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- self schema generate
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- self schema check
