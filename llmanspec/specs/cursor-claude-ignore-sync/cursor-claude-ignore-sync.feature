# language: zh-CN
# capability: cursor-claude-ignore-sync
# purpose: 规范 cursor 与 claude ignore 配置的统一解析与 union 同步行为。
# scope: llmanspec/specs/cursor-claude-ignore-sync

功能: cursor-claude-ignore-sync

  @req:r19 @human
  场景: git 守卫、交互式模式与 x 子命令快捷方式
    - 对应 spec: cursor-claude-ignore-sync — 系统 MUST 强制检查 git root（非 git 目录报错， --force 可绕过）；MUST 提供交互式模式选择 targets/预览/确认；SHOULD 通过 llman x 子命令 提供快捷方式（cc→claude-shared，cursor→cursor）。

  @req:r50 @human
  场景: include 规则解析与 Claude Code settings 读写策略
    - 对应 spec: cursor-claude-ignore-sync — 系统 MUST 解析 gitignore 风格的 include（!pattern） 规则并稳定写回；MUST 解析/更新 Claude Code settings（仅 permissions.deny 的 Read(...)）， 尽量保留 JSONC 注释（best-effort），并保留非 Read deny 规则。

  @req:r74 @human
  场景: ignore 配置统一解析并以并集同步
    - 对应 spec: cursor-claude-ignore-sync — 系统 MUST 提供 llman tool sync-ignore 命令， 以 union（并集）方式统一解析并同步 ignore 配置到选定 targets（OpenCode .ignore / Cursor .cursorignore / Claude Code .claude/settings*.json 的 permissions.deny Read）。
