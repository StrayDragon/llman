# language: zh-CN
# capability: tool-agents-md-management
# purpose: 规范 `llman tool agents-md` 命令组（scan/clean/revert）对项目内 agent init 文件（AGENTS.md、CLAUDE.md、.cursor/ 等）的登记、按需清理与从主干恢复行为，解决个人 PR 开发时的 init 文件语义过期冲突。
# scope: llmanspec/specs/tool-agents-md-management

功能: tool-agents-md-management

  @req:r121 @human
  场景: scan 发现与登记
    - 对应 spec: tool-agents-md-management — `llman tool agents-md scan` MUST 递归扫描项目内 agent init 文件并输出相对项目根的路径列表；扫描文件名清单来源为内置默认 `[AGENTS.md, CLAUDE.md, GEMINI.md, .cursorrules, .cursor/, .claude/, .windsurfrules, .github/copilot-instructions.md]` 经 global config `tools.agents-md` 覆盖（非多级并集）；传入 `--upsert-project-configs` 时 MUST 将发现的路径（文件或目录名）写入项目 `.llman/config.yaml` 的 `tools.agents-md.files` 段，文件不存在则创建。

  @req:r122 @human
  场景: clean 删除与默认分支安全门禁
    - 对应 spec: tool-agents-md-management — `llman tool agents-md clean` MUST 读取项目 config `tools.agents-md.files` 清单并将目录项展开为 git-tracked 文件逐个删除；默认 dry-run（仅预览），`--commit` 时 MUST 单次 `git add <files>` 并以固定前缀 `chore(agents-md):` 提交；当前分支为默认分支（main/master）时 MUST 拒绝 `--commit` 执行，除非显式传入 `--force`。

  @req:r123 @human
  场景: revert 从默认分支恢复
    - 对应 spec: tool-agents-md-management — `llman tool agents-md revert` MUST 将清单（含展开后的目录内文件）逐个从默认分支（探测顺序 origin/HEAD → origin/main → origin/master → main → master）`git checkout <default> -- <file>` 恢复到工作区；传入 `--commit` 时 MUST 自动创建分支并提交恢复结果。
