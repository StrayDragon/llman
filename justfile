default:
    @just --list

# 构建项目 (dev)
build:
    cargo build

# 构建发布版本 (dev)
build-release:
    cargo build --release

# 运行项目 (dev)
run *args:
    LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- {{args}}

# 使用测试配置运行
run-prod *args:
    cargo run -- {{args}}

# 运行测试 (dev)
test:
    cargo test

# 代码格式化 (dev)
fmt:
    cargo fmt

# 代码检查 (dev)
clippy:
    cargo clippy -- -D warnings

# 清理构建产物 (dev)
clean:
    cargo clean

# 安装到本地
install:
    cargo install --path .

# 检查一条龙 (dev)
check: fmt clippy test

# 创建新的规则模板 (dev)
create-dev-template name content:
    @echo "{{content}}" > ./artifacts/testing_config_home/prompt/cursor/{{name}}.mdc
    @echo "✅ 模板 {{name}} 已创建"
