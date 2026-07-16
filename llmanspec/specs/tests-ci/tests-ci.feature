# language: zh-CN
# managed by llman sdd partition-migrate
功能: tests-ci

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

  @req:r1
  场景: check-all 执行 schema 校验
    假如 开发者运行 just check-all
    当 check-all 执行其步骤序列
    那么 just check-schemas 会被执行
