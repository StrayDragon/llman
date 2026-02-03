default:
    @just --list

# =============================================================================
# 构建和运行命令
# =============================================================================

# 构建项目
build:
    cargo +nightly build

# 构建发布版本
build-release:
    cargo +nightly build --release

# 运行项目（使用测试配置）
run *args:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- {{args}}

# 使用生产配置运行
run-prod *args:
    cargo +nightly run -- {{args}}

# 安装到本地
install:
    cargo +nightly install --path .

# 清理构建产物
clean:
    cargo clean

# =============================================================================
# 测试命令
# =============================================================================

# 运行测试
test:
    cargo +nightly test

# =============================================================================
# 代码质量检查
# =============================================================================

# 代码格式化
fmt:
    cargo +nightly fmt

# 检查代码格式化（不修改文件）
fmt-check:
    cargo +nightly fmt --all -- --check

# 代码检查（clippy，包含重要警告）
lint:
    cargo +nightly clippy -- -D warnings

# 快速编译检查
check-compile:
    cargo +nightly check --all-targets

# 文档检查
doc-check:
    cargo +nightly doc --no-deps --all-features --document-private-items

# 核心检查（格式化检查 + lint + 测试）
check: fmt-check lint test

# 完整检查（核心检查 + 文档 + release构建 + SDD模板检查）
check-all: check doc-check build-release check-sdd-templates check-schemas

# 别名：完整检查
alias qa := check-all

# =============================================================================
# 工具命令
# =============================================================================

# 创建新的规则模板
create-dev-template name content:
    @echo "{{content}}" > ./artifacts/testing_config_home/prompt/cursor/{{name}}.mdc
    @echo "✅ 模板 {{name}} 已创建"

# 检查 i18n 状态
check-i18n:
    ./scripts/check-i18n.sh

# 检查 SDD 模板版本与本地化一致性
check-sdd-templates:
    ./scripts/check-sdd-templates.py

# 检查配置 schema
check-schemas:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- self schema check
