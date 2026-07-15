# language: zh-CN
# 对应 spec: prompts-management — llman prompts MUST 仅交互式运行并作为 llman x <app> prompts
# 编排入口；prompts 主命令保留 prompt 别名；支持 cursor/codex/claude-code 三类 app 按 app 隔离；
# --scope 解析为目标集合（global|project）。
功能: prompts 编排入口、app 隔离与 scope 解析
  场景: 非交互输出迁移提示
    假如 用户运行 llman prompts --no-interactive
    当 命令执行
    那么输出提示用户使用 llman x cursor/codex/claude-code prompts

  场景: 使用别名调用等价
    假如 用户运行 llman prompt --no-interactive
    当 命令执行
    那么行为与 llman prompts --no-interactive 等价

  场景: app 维度隔离模板
    假如 用户分别对 cursor 与 codex prompts upsert 写入并用相同 name
    当 命令执行
    那么两者模板互不覆盖
    而且cursor prompts list 不显示 codex 模板

  场景: 重复参数选择双 scope
    假如 用户运行 llman x codex prompts gen --scope global --scope project
    当 命令执行
    那么同时处理全局与项目目标

  场景: 逗号列表选择双 scope
    假如 用户运行 llman x claude-code prompts gen --scope global,project
    当 命令执行
    那么同时处理全局与项目目标

  场景: cursor 传入不支持的 scope 报错
    假如 用户运行 llman x cursor prompts gen --scope global
    当 命令执行
    那么返回错误并提示 cursor 仅支持 project

  场景: 传入已移除的旧 scope 报错
    假如 用户运行 llman x codex prompts gen --scope user
    当 命令执行
    那么返回错误
