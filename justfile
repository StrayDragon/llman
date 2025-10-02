default:
    @just --list

# 构建项目 (dev)
build:
    cargo +nightly build

# 构建发布版本 (dev)
build-release:
    cargo +nightly build --release

# 运行项目 (dev)
run *args:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- {{args}}

# 使用测试配置运行
run-prod *args:
    cargo +nightly run -- {{args}}

# 运行测试 (dev)
test:
    cargo +nightly test

# 代码格式化 (dev)
fmt:
    cargo +nightly fmt

# 代码检查 (dev)
lint:
    cargo +nightly clippy -- -D warnings

# nightly 版本代码检查 (dev)
lint-nightly:
    cargo +nightly clippy -- -D warnings

# 清理构建产物 (dev)
clean:
    cargo clean

# 安装到本地
install:
    cargo +nightly install --path .

# 检查一条龙 (dev)
check: fmt lint-nightly test

# 创建新的规则模板 (dev)
create-dev-template name content:
    @echo "{{content}}" > ./artifacts/testing_config_home/prompt/cursor/{{name}}.mdc
    @echo "✅ 模板 {{name}} 已创建"

# 检查 i18n 状态 (dev)
check-i18n:
    ./scripts/check-i18n.sh
