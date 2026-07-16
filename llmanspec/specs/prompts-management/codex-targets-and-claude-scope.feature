# language: zh-CN
# 对应 spec: prompts-management — codex prompts 支持 --target project-doc|prompts（默认 prompts），
# --override 仅对 project-doc 生效；codex 与 claude-code 均支持 global/project 双 scope 写入；
# claude memory 注入用托管块策略保留用户内容；读取失败时不静默覆盖。
功能: codex target 选择与 claude-code 双 scope 注入
  场景: 默认 target 为 prompts
    假如 用户运行 llman x codex prompts gen --template {tpl}
    当 命令执行
    而且 那么按 --target prompts 处理

  场景: override 仅对 project-doc 生效
    假如 用户运行 llman x codex prompts gen --target prompts --override --template {tpl}
    当 命令执行
    而且 那么返回错误

  场景: 生成 codex 全局 custom prompt
    假如 用户运行 llman x codex prompts gen --target prompts --scope global --template {tpl}
    当 命令执行
    而且 那么创建或更新 $CODEX_HOME/prompts/{tpl}.md

  场景: 生成 codex 项目 custom prompt
    假如 用户运行 llman x codex prompts gen --target prompts --scope project --template {tpl}
    当 命令执行
    而且 那么创建或更新 <repo_root>/.codex/prompts/{tpl}.md

  场景: 生成 codex 全局 project-doc
    假如 用户运行 llman x codex prompts gen --target project-doc --scope global --template {tpl}
    当 命令执行
    而且 那么创建或更新 $CODEX_HOME/AGENTS.md

  场景: 生成 codex override project-doc
    假如 用户运行 llman x codex prompts gen --target project-doc --scope global --override --template {tpl}
    当 命令执行
    而且 那么创建或更新 $CODEX_HOME/AGENTS.override.md

  场景: 生成 claude-code 全局 memory
    假如 用户运行 llman x claude-code prompts gen --scope global --template {tpl}
    当 命令执行
    而且 那么创建或更新 ~/.claude/CLAUDE.md

  场景: 生成并保留用户自定义内容
    假如 项目 CLAUDE.md 已含用户手写内容
    当 用户运行 llman x claude-code prompts gen --scope project --template {tpl}
    而且 那么仅更新托管块内容
    而且 而且不删除或改写用户手写内容

  场景: 既有 CLAUDE.md 不可读时不静默覆盖
    假如 <repo_root>/CLAUDE.md 存在但无法作为 UTF-8 读取
    当 用户运行 llman x claude-code prompts gen --scope project
    而且 那么返回错误
    而且 而且文件未被修改
