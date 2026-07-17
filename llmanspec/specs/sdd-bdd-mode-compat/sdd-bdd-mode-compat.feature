# language: zh-CN
# managed by llman sdd (Partitioned SSOT harness)
功能: sdd-bdd-mode-compat

  @req:r6
  场景: @req 指向缺失 requirement 时 validate --strict 失败
    假如 已初始化含无效 @req 的 sdd 项目且 bdd 配置为 on
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 stderr 包含 @req

  @req:r83
  场景: BDD-off 时 validate 静默忽略 .feature 文件
    假如 llman 二进制已构建
    当 运行 llman sdd validate sample --strict
    那么 退出码为零

  @req:r78
  场景: BDD-on 时 index rebuild 编入 feature 派生的 scenario
    假如 llman 二进制已构建
    当 运行 llman sdd index rebuild
    那么 stdout 包含 rebuilt

  @req:r86
  场景: global-req-collision-default
    假如 已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate --specs --no-check
    那么 退出码非零且 stderr 包含 next-req-id

  @req:r86
  场景: global-req-collision-strict
    假如 已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate --all --strict --no-check
    那么 退出码非零且 stderr 包含 Global duplicate req_id

  @req:r85
  场景: partition-migrate --dry-run 只打印计划
    假如 已初始化 sdd 项目且 bdd 配置为 on
    当 在非交互终端运行 llman sdd project partition-migrate --dry-run
    那么 stdout 包含 dry-run

  @req:r5
  场景: 双写可执行 GWT 时 validate --strict 失败
    假如 已初始化含可执行双写的 sdd 项目且 bdd 配置为 on
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 stderr 包含 dual-write
