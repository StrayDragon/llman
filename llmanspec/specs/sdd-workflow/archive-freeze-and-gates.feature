# language: zh-CN
# 对应 spec: sdd-workflow r16-r20 — archive 子命令组（run 合并 delta 并移动到 archive）；
# archive gate 基于 pending 阻塞；dry-run 预览；freeze/thaw 冷备；模板版本；归档前置校验；
# --force 隐藏；--skip-specs；staleness 警告视为失败。
功能: 归档流程、前置校验与冻结解冻
  场景: archive pending 阻塞
    假如 tasks.md 有 pending
    当 执行 archive
    而且 那么归档被阻塞

  场景: archive 全部 completed 成功
    假如 tasks.md 全部 completed
    当 执行 archive
    而且 那么归档成功

  场景: completion ratio 计入全部 task
    假如 全部 completed 且 min_completion_ratio 启用
    当 执行 archive
    那么 ratio 为 completed/total

  场景: 归档 dry-run 不改动文件系统
    假如 用户执行 llman sdd archive <change-id> --dry-run
    当 命令执行
    而且 那么输出预览信息且文件系统无改动

  场景: 单文件冻结
    假如 用户执行 llman sdd archive freeze
    当 命令执行
    而且 那么生成或更新 freezed_changes.7z.archived

  场景: 再次冻结复用同一文件
    假如 冷备归档文件已存在时用户再次执行 freeze
    当 命令执行
    而且 那么继续写入同一文件
    而且 而且历史内容在后续 thaw 中仍可恢复

  场景: 解冻到默认目录
    假如 用户执行 llman sdd archive thaw
    当 命令执行
    而且 那么内容恢复到 .thawed/

  场景: 校验失败阻止归档
    假如 用户执行 archive 且任一 spec 校验失败
    当 命令执行
    而且 那么退出非零且不写入/移动任何文件

  场景: staleness 警告视为失败
    假如 staleness 状态为 STALE 或 WARN
    当 执行归档
    而且 那么归档失败并提示修复

  场景: force 参数隐藏
    假如 用户执行 llman sdd archive --help
    当 查看帮助
    而且 那么帮助输出不包含 --force

  场景: skip-specs 跳过校验
    假如 用户执行 llman sdd archive <change-id> --skip-specs
    当 命令执行
    而且 那么不执行归档前的 spec 校验

  场景: 错误提示不引导绕过
    假如 归档因校验失败而中止
    当 查看输出
    而且 那么仅提示修复校验问题
    而且 而且不提示 --force

  @executable @req:r92
  场景: freeze-list-无归档文件时提示并退出零
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd archive freeze --list
    那么 退出码为零
    而且 stdout 包含 No freeze archive found
