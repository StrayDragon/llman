# language: zh-CN
# managed by llman sdd (Partitioned SSOT harness)
功能: global-req-id

  @executable @req:r86
  场景: global-req-collision-default
    假如 已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate --specs --no-check
    那么 退出码非零且 stderr 包含 next-req-id

  @executable @req:r86
  场景: global-req-collision-strict
    假如 已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate --all --strict --no-check
    那么 退出码非零且 stderr 包含 Global duplicate req_id
