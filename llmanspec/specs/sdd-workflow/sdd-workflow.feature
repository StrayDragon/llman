# language: zh-CN
# managed by llman sdd (Partitioned SSOT harness)
功能: sdd-workflow

  @req:r88
  场景: add-req-rejects-global-collision
    假如 已初始化含已占用全局 req_id 的 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd spec add-req sample occupied-id --title t --statement "MUST keep unique"
    那么 退出码非零且 stderr 包含 occupied-id

  @req:r87
  场景: next-req-id-json
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd spec next-req-id --json
    那么 退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId

  @req:r89
  场景: resolve-req-json
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd spec resolve-req r1 --json
    那么 退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId 且含 JSON 键 capability
