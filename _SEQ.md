# fix-* Changes Execution Sequence

用于按推荐顺序逐个落地 `openspec/changes/fix-*` 的执行清单，避免遗漏。

## 全局流程（每个 change 都走一遍）
- [x] 阅读 `proposal.md` / `design.md`（如有）/ `tasks.md`，确认范围与验收标准
- [x] 按 `tasks.md` 顺序实现（最小改动、只修复约定行为）
- [x] 为新增/修改行为补齐测试
- [x] 更新该 change 的 `tasks.md`：全部完成后逐项勾选为 `- [x]`
- [x] 运行 `openspec validate <id> --strict --no-interactive`
- [x] 运行 `just test`

## 推荐执行顺序（拓扑 + 优先级）
1. [x] `fix-sdd-command-safety-and-flags`（P0：安全/高影响）
   - `openspec/changes/fix-sdd-command-safety-and-flags/tasks.md`
2. [x] `fix-tool-rm-useless-dirs-gitignore-and-protection`（P0：潜在误删/误遍历）
   - `openspec/changes/fix-tool-rm-useless-dirs-gitignore-and-protection/tasks.md`
3. [x] `fix-cursor-export-interactive-selection`（P0：正确性“选 A 导出 B”）
   - `openspec/changes/fix-cursor-export-interactive-selection/tasks.md`
4. [x] `fix-prompts-command-edge-cases`（P0：一致性/安全覆盖/可脚本化）
   - `openspec/changes/fix-prompts-command-edge-cases/tasks.md`
5. [x] `fix-skills-command-robustness`（P1：幂等/崩溃安全/交互取消）
   - `openspec/changes/fix-skills-command-robustness/tasks.md`
6. [x] `fix-claude-code-command-args-and-security`（P1：quote-aware args + 安全匹配一致性）
   - `openspec/changes/fix-claude-code-command-args-and-security/tasks.md`
7. [x] `fix-self-schema-check-and-paths`（P1：schema 工具正确性 + 路径发现）
   - `openspec/changes/fix-self-schema-check-and-paths/tasks.md`
8. [x] `fix-self-completion-noninteractive-install`（P2：自动化安装护栏）
   - `openspec/changes/fix-self-completion-noninteractive-install/tasks.md`
9. [x] `fix-codex-command-editor-handling`（P2：editor 可用性；复用 quote-aware split）
   - `openspec/changes/fix-codex-command-editor-handling/tasks.md`
10. [x] `fix-tool-clean-comments-cli-and-rules`（P2/P3：工具可用性/性能）
    - `openspec/changes/fix-tool-clean-comments-cli-and-rules/tasks.md`

## Change Execution Sequence（2026-02-06）

- [x] fix-sdd-command-safety-and-flags
- [x] fix-tool-rm-useless-dirs-gitignore-and-protection
- [x] fix-cursor-export-interactive-selection
- [x] fix-prompts-command-edge-cases
- [x] fix-skills-command-robustness
- [x] fix-claude-code-command-args-and-security
- [x] fix-self-schema-check-and-paths
- [x] fix-self-completion-noninteractive-install
- [x] fix-codex-command-editor-handling
- [x] fix-tool-clean-comments-cli-and-rules

## 关键决策备忘（已在对应 change 文档中固化）
- prompts project-scope 找不到 git root：交互模式提示是否强制；非交互需显式 `--force` 才允许以 `cwd` 作为 root 继续写入。
- skills 冲突提示取消：整体 abort（不产生任何写入/部分变更）。
- quote-aware parser：统一采用确定的“shell-like words split”语义；未闭合引号/非法转义视为输入错误并返回清晰提示（交互可重试）。
- schema apply header：只规范化文件顶部的 header 区域，最小侵入；真实 YAML 文件存在但不可读/不可解析时，`schema check` 必须失败（不回退 defaults）。
