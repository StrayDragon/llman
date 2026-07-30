# language: zh-CN
# 对应 spec: tool-agents-md-management — `llman tool agents-md` 命令组 MUST 提供 scan/clean/revert
# 三个子命令，登记、按需清理与从主干恢复 agent init 文件（AGENTS.md / CLAUDE.md / .cursor/ 等）。

功能: agents-md 命令组登记清理与恢复 agent init 文件

  @req:r121
  场景: scan 列出项目内已发现的 agent init 文件
    假如 项目内存在 AGENTS.md 与 CLAUDE.md 文件
    当 运行 llman tool agents-md scan
    那么 退出码为 0
    那么 stdout 包含 AGENTS.md
    那么 stdout 包含 CLAUDE.md

  @req:r121
  场景: scan 带 upsert 写入项目 config 清单
    假如 项目内存在 AGENTS.md 文件且 .llman/config.yaml 不存在
    当 运行 llman tool agents-md scan --upsert-project-configs
    那么 退出码为 0
    那么 项目 .llman/config.yaml 含 agents-md.files 段

  @req:r121
  场景: scan 文件名清单经 global config 覆盖内置默认
    假如 global config 的 tools.agents-md.files 仅含 AGENTS.md
    当 项目内存在 AGENTS.md 与 CLAUDE.md 且运行 llman tool agents-md scan
    那么 stdout 包含 AGENTS.md
    那么 stdout 不包含 CLAUDE.md

  @req:r122
  场景: clean 默认 dry-run 仅预览不删除
    假如 项目 config 清单含 AGENTS.md 且文件存在于工作区
    当 运行 llman tool agents-md clean
    那么 退出码为 0
    那么 AGENTS.md 仍存在于工作区

  @req:r122
  场景: clean 在默认分支上带 commit 被拒绝
    假如 当前分支为 main 且项目 config 清单含 AGENTS.md
    当 运行 llman tool agents-md clean --commit
    那么 退出码为 1
    那么 stderr 包含 默认分支

  @req:r122
  场景: clean 在默认分支上带 commit 与 force 仍执行
    假如 当前分支为 main 且项目 config 清单含 AGENTS.md
    当 运行 llman tool agents-md clean --commit --force
    那么 退出码为 0

  @req:r122
  场景: clean 目录清单项展开为 tracked 文件逐个删除
    假如 项目 config 清单含 .cursor/ 目录且其下有 tracked 文件
    当 运行 llman tool agents-md clean --yes
    那么 退出码为 0
    那么 .cursor 目录下 tracked 文件已被删除

  @req:r123
  场景: revert 从默认分支逐个恢复清单文件
    假如 默认分支含 AGENTS.md 主干版本且当前工作区该文件已被删除
    当 运行 llman tool agents-md revert --yes
    那么 退出码为 0
    那么 AGENTS.md 恢复为默认分支版本

  @req:r123
  场景: revert 带 commit 自动创建分支并提交
    假如 当前分支为非默认分支且清单含 AGENTS.md
    当 运行 llman tool agents-md revert --commit
    那么 退出码为 0
    那么 新分支已创建并含恢复提交
