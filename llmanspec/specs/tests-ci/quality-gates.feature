# language: zh-CN
# 对应 spec: tests-ci r1 — CI MUST 在锁定的 nightly 基线上运行格式检查与
# clippy(-D warnings)，并运行 release 构建检查。
功能: CI 质量门在锁定 nightly 基线上运行
  @req:r1
  场景: check 作业使用锁定 nightly 基线并运行 check-all
    假如 CI 在 main 分支运行
    当 执行 check 作业
    那么 使用仓库锁定的 nightly 基线
    而且 执行 just check-all 或等价的基于 nightly 的检查序列

  @req:r1
  场景: build 作业在锁定 nightly 基线上运行 release 构建
    假如 CI 在 main 分支运行
    当 执行 build 作业
    那么 使用仓库锁定的 nightly 基线
    而且 执行 just build-release

  @req:r1
  场景: 测试代码不触发 clippy 警告
    假如 运行 cargo +nightly clippy -- -D warnings
    当 clippy 扫描测试代码
    那么 测试代码不发出 len_zero 或类似警告
