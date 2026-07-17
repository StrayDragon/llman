# language: zh-CN
# 对应 spec: sdd-workflow r7-r11,r26-r27 — OpenSpec 双向互转（--style 必填，仅 openspec）；
# 互转执行安全门禁（dry-run + 双确认）；元数据补齐；list/show 的默认/specs/changes/json 行为；
# 交互/非交互提示；ID 作为标识符拒绝路径穿越；list --specs 与 --changes 互斥。
功能: OpenSpec 互转、列表查看与标识符安全
  场景: style 参数缺失报错
    假如 用户执行 import 或 export 且未传 --style
    当 命令执行
    而且 那么返回非零并提示需要 --style openspec

  场景: style 参数非法报错
    假如 用户执行 import --style unknown
    当 命令执行
    而且 那么返回非零并提示仅支持 openspec

  场景: 非标准目录复制并警告
    假如 源目录含非标准目录（如 explorations/）
    当 命令执行
    而且 那么输出 warning
    而且 而且按相对路径复制到目标侧

  场景: 目标冲突即失败
    假如 目标目录存在计划写入的同名文件
    当 命令执行
    而且 那么返回非零并中止

  场景: 非交互环境仅演练
    假如 用户在非交互环境执行 export --style openspec
    当 命令执行
    而且 那么输出 dry-run 计划并返回非零

  场景: 交互双确认后执行
    假如 用户在交互环境执行 import --style openspec 且通过双确认
    当 命令执行
    而且 那么执行实际写入

  场景: 默认不删除旧目录
    假如 迁移写入成功后进入删除旧目录提示
    当 展示选项
    而且 那么默认为不删除

  场景: 默认列出变更排除 archive
    假如 用户执行 llman sdd list
    当 命令执行
    而且 那么输出 changes/ 下的变更目录（排除 archive）

  场景: 列出 specs
    假如 用户执行 llman sdd list --specs
    当 命令执行
    而且 那么输出 specs/ 下的 spec 目录

  场景: show 自动识别与歧义处理
    假如 用户执行 llman sdd show <item-name> 且未指定 --type
    当 命令执行
    而且 那么自动识别 change/spec
    而且 而且若同时匹配则报错并提示使用 --type

  场景: show 非交互提示语
    假如 用户在非交互环境执行 llman sdd show
    当 命令执行
    而且 那么输出 Nothing to show 提示并退出码为 1

  场景: validate 非交互提示语
    假如 用户在非交互环境执行 llman sdd validate
    当 命令执行
    而且 那么输出 Nothing to validate 提示并退出码为 1

  场景: 拒绝路径穿越 id
    假如 用户运行 llman sdd archive ../oops
    当 命令执行
    而且 那么返回错误
    而且 而且不移动或修改任何文件

  场景: 同时传入 specs 与 changes 报错
    假如 用户运行 llman sdd list --specs --changes
    当 命令执行
    而且 那么返回错误并以非零退出
