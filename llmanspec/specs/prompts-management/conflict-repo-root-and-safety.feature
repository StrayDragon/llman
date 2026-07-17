# language: zh-CN
# 对应 spec: prompts-management — 冲突与覆盖策略一致（托管目标交互确认/非交互需 --force；
# 非托管文件二次确认）；list 仅展示可读取模板；非交互 rm 需 --yes；
# project scope 经 repo root 发现解析；无 git root 时需 --force。
功能: 冲突策略、模板列举、删除确认与 project scope repo root 解析
  @req:r55
  场景: 交互模式下 custom prompt 覆盖确认
    假如 目标 custom prompt 文件已存在且终端可交互
    当 命令执行
    而且 那么提示确认是否覆盖

  @req:r55
  场景: 交互模式下非托管文件触发二次确认
    假如 目标文件存在且不含 llman 托管块，用户在交互模式运行 gen --target project-doc
    当 命令执行
    而且 那么执行二次确认

  @req:r55
  场景: 非交互模式下非托管文件未提供 force 被拒
    假如 目标文件存在且不含托管块，终端不可交互且未提供 --force
    当 命令执行
    而且 那么拒绝该目标写入并返回错误

  @req:r55
  场景: 模板目录含混合扩展名时仅展示可读模板
    假如 模板目录同时含支持模板与不相关文件（如备份）
    当 用户运行 llman x <app> prompts list
    而且 那么仅展示可读取模板

  @req:r55
  场景: 非交互 rm 未提供 yes 被拒
    假如 终端不可交互且用户运行 llman x cursor prompts rm --name foo
    当 命令执行
    而且 那么返回非零错误并提示需要 --yes

  @req:r55
  场景: 非交互 rm 提供 yes 删除
    假如 终端不可交互且用户运行 llman x cursor prompts rm --name foo --yes
    当 命令执行
    而且 那么模板被删除
    而且 而且不出现交互提示

  @req:r55
  场景: 在 repo 子目录运行 codex project prompts
    假如 用户在 repo 子目录运行 codex prompts gen --target prompts --scope project --template {tpl}
    当 命令执行
    而且 那么输出写入 <repo_root>/.codex/prompts/{tpl}.md

  @req:r55
  场景: 在 repo 子目录运行 codex project-doc
    假如 用户在 repo 子目录运行 codex prompts gen --target project-doc --scope project --template {tpl}
    当 命令执行
    而且 那么输出写入 <repo_root>/AGENTS.md

  @req:r55
  场景: 在 repo 子目录运行 cursor project scope
    假如 用户在 repo 子目录运行 cursor prompts gen --scope project --template {tpl}
    当 命令执行
    而且 那么输出写入 <repo_root>/.cursor/rules/

  @req:r55
  场景: 非交互无 git root 且仅 project scope 被拒
    假如 终端不可交互、当前目录不在 git repo 内且仅请求 project scope
    当 用户运行 gen --scope project
    而且 那么返回非零错误并提示需要 --force
    而且 而且不写入 project 目标

  @req:r55
  场景: 非交互无 git root 但选择 global+project 时尝试 global
    假如 终端不可交互、不在 git repo 内且运行 --scope global --scope project
    当 命令执行
    而且 那么尝试写入 global 目标

  @req:r55
  场景: 非交互无 git root 但提供 force 时视 cwd 为 root
    假如 终端不可交互、不在 git repo 内且提供 --force
    当 用户运行 gen --scope project --force
    而且 那么将 cwd 视为 project root 并写入 <cwd>/.codex/prompts/

  @req:r55
  场景: 交互无 git root 且用户拒绝强制时安全退出
    假如 终端可交互、不在 git repo 内且用户在提示中选择不强制执行
    当 命令执行
    而且 那么安全退出
    而且 而且不写入任何文件
