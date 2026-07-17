# language: zh-CN
# 对应 spec: dependency-upgrade-workflow — 维护者 MUST 以 lockfile 优先的顺序
# 在锁定的 nightly 基线下升级依赖；仅在 lockfile 更新不足时才改 Cargo.toml 约束。
功能: 依赖升级采用 lockfile 优先顺序
  @req:r1
  场景: 升级开始时先更新 lockfile 并跑校验
    假如 维护者开始为本仓库执行依赖升级
    当 维护者先更新 {lockfile} 并运行校验
    那么 在改动 {manifest} 中的依赖版本约束前先完成 lockfile 更新

  @req:r1
  场景: lockfile 更新因版本约束不足时才最小化改动 manifest
    假如 某次升级需要 manifest 约束改动才能编译或通过校验
    当 维护者应用约束改动
    那么 仅应用所需的边界更新
    而且 以基于 nightly 的校验验证结果
