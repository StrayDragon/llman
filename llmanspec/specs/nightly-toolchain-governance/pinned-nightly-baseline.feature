# language: zh-CN
# 对应 spec: nightly-toolchain-governance — 仓库 MUST 在 {toolchain_file} 中定义
# 单一锁定的 nightly 基线，作为权威的本地构建工具链。
功能: 锁定 nightly 作为单一构建基线
  @req:r54
  场景: 开发者运行构建或检查命令时工具链解析为锁定基线
    假如 开发者在仓库中运行构建或检查命令
    当 工具链被解析
    那么 生效的 Rust 工具链为 {toolchain_file} 定义的锁定 nightly 基线
