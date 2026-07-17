# language: zh-CN
# 对应 spec: cli — 仅需全局配置的子命令才做 dev-project config 守卫；authoring 命令统一命名
# （add-req/remove-req/rename-req）；非核心命令归入 sdd project 子组；重命名命令提供废弃别名；
# archive 必须显式子命令；show 提供组合输出选项。
功能: 配置守卫范围与命令结构一致性
  场景: sdd list 在 llman 项目目录无需 config-dir
    假如 用户在 llman 项目目录运行 llman sdd list 且不带 --config-dir
    当 命令执行
    而且 那么成功执行且无 dev-project config 错误

  场景: add-req 命令可用
    假如 用户运行 llman sdd spec add-req
    当 命令执行
    而且 那么执行 add requirement 命令

  场景: add-requirement 旧别名仍工作
    假如 用户运行 llman sdd spec add-requirement
    当 命令执行
    而且 那么经别名执行 add requirement 命令

  场景: project 子组 import
    假如 用户运行 llman sdd project import
    当 命令执行
    而且 那么执行 import 命令

  场景: project 子组 migrate
    假如 用户运行 llman sdd project migrate
    当 命令执行
    而且 那么执行 migrate 命令

  场景: 废弃别名仍工作并告警
    假如 用户运行 llman sdd import
    当 命令执行
    而且 那么显示废弃告警
    而且 而且执行 import

  场景: archive 必须显式子命令
    假如 用户运行 llman sdd archive {id}
    当 命令执行
    而且 那么返回错误并要求 archive run {id}

  场景: show 组合输出选项
    假如 用户运行 llman sdd show {id} --output json,deltas
    当 命令执行
    而且 那么输出仅含 deltas 的 JSON
