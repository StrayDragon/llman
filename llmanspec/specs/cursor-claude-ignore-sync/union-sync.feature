# language: zh-CN
# 对应 spec: cursor-claude-ignore-sync — 系统 MUST 提供 llman tool sync-ignore 命令，
# 以 union（并集）方式统一解析并同步 ignore 配置到选定 targets（OpenCode .ignore /
# Cursor .cursorignore / Claude Code .claude/settings*.json 的 permissions.deny Read）。
功能: ignore 配置统一解析并以并集同步
  @req:r1
  场景: 默认 dry-run 预览不写入
    假如 当前目录位于一个 git repo 内
    当 用户执行 llman tool sync-ignore
    那么 系统自动发现项目内存在的 sources
    而且 而且默认仅预览不写入

  @req:r1
  场景: --yes 应用写入并自动创建缺失 targets
    假如 当前目录位于一个 git repo 内
    当 用户执行 llman tool sync-ignore --yes
    那么 系统把 union 结果写入/创建默认 targets
    而且 而且默认 targets 含 .ignore、.cursorignore、.claude/settings.json

  @req:r1
  场景: --target 限制输出目标
    假如 用户执行 llman tool sync-ignore --target {target}
    当 系统写入
    那么 仅写入/创建该 target 对应文件
