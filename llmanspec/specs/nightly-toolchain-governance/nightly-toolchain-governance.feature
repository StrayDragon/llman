# language: zh-CN
# managed by llman sdd partition-migrate
功能: nightly-toolchain-governance

  @req:r1
  场景: 升级 nightly 日期须通过质量门
    假如 {toolchain_file} 被改为更新的 nightly 日期
    当 维护者评估该 bump
    那么 改动通过基于 nightly 的格式、lint、测试与 release 构建检查

  @req:r1
  场景: 新 nightly 引入阻断性回归时可回退
    假如 nightly 升级后出现阻断合并或发布的回归
    当 维护者回退
    那么 可恢复先前的锁定 nightly 基线
    而且 无需重写无关代码即可恢复绿色构建

  @req:r1
  场景: 开发者运行构建或检查命令时工具链解析为锁定基线
    假如 开发者在仓库中运行构建或检查命令
    当 工具链被解析
    那么 生效的 Rust 工具链为 {toolchain_file} 定义的锁定 nightly 基线
