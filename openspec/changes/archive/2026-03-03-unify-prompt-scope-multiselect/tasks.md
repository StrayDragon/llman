## 1. Scope 参数模型重构

- [x] 1.1 将 `--scope` 从单值枚举改为集合解析（支持重复参数与逗号列表），统一内部 `global|project` 表达。
- [x] 1.2 为各 app 建立可用 scope 集合校验（codex/claude-code: global+project；cursor: project），并补齐错误提示与交互文案。
- [x] 1.3 移除 `user|all` 旧语义与相关分支；旧值输入直接报错，不提供迁移提示。

## 2. Codex 目标模型与写入实现

- [x] 2.1 为 codex 增加 `--target agents|prompts`（默认 `prompts`），并约束 `--override` 仅对 `agents` 目标生效。
- [x] 2.2 实现 codex `--target prompts`：按 scope 写入 `$CODEX_HOME/prompts/<name>.md` 与 `<repo_root>/.codex/prompts/<name>.md`（每个模板独立文件）。
- [x] 2.3 实现 codex `--target agents`：按 scope 注入到 `$CODEX_HOME/AGENTS*.md` 与 `<repo_root>/AGENTS*.md`（托管块聚合，模板名仅作为区段标题）。
- [x] 2.4 统一 codex/claude-code/cursor 的“逐目标处理”执行模型，避免 scope/target 间短路，且任一目标失败时整体非 0 退出。

## 3. 冲突与确认策略

- [x] 3.1 托管块注入目标（codex agents、claude-code memory）在交互模式下对非托管文件执行二次确认；非交互模式下必须 `--force`。
- [x] 3.2 完全托管目标（codex prompts、cursor rules）保持单次覆盖确认；非交互覆盖需 `--force`。

## 4. 测试与回归覆盖

- [x] 4.1 更新 `src/prompt.rs` 单元测试：scope 解析、codex target/override 解析、目标路径映射、project root 解析、非短路行为。
- [x] 4.2 新增/更新失败路径测试：cursor 不支持 global、旧 scope 值报错、`--override` 但未选择 agents、无 git root 且未 `--force` 的 project 失败。
- [x] 4.3 新增注入测试：codex agents 托管块更新（保留非托管内容、重复执行幂等）。
- [x] 4.4 运行 `openspec validate unify-prompt-scope-multiselect --strict --no-interactive` 并修复校验问题。

## 5. 文档与发布说明

- [x] 5.1 更新 prompts 相关文档与示例命令，统一为 `global|project` 多选写法，并补齐 codex `--target`/`--override` 用法。
- [x] 5.2 在变更说明中标注 BREAKING：`user/all` 移除、codex scope/target 语义升级。
