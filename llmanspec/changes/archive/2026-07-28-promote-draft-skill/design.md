# Design: promote-draft-skill

## 决策：重命名而非新增

选择把 `llman-sdd-new-change` **重命名**为 `llman-sdd-draft`（而非新建第三个 skill + 废弃 new-change），理由：

- 避免留下三个语义重叠的 skill（draft / draft-propose / new-change），增加解释成本。
- `new-change` 当前是 optional 且基本无人用，重命名零用户感知成本。
- 重命名后职责单一化（draft shell only），与 propose（完整提案）边界清晰。

## 向后兼容：自愈迁移（无需显式迁移代码）

重命名后，已 init 项目的迁移路径依赖现有的 `cleanup_stale_skills` + `resolve_optional_skills` 机制，**自愈**：

| 残留状态 | 机制 | 结果 |
|---|---|---|
| `.agents/skills/llman-sdd-new-change/` 目录 | `cleanup_stale_skills`（`update_skills.rs:69-100`）：不在候选集即删 | 自动删除 |
| `config.yaml` 里 `extra_skills: [llman-sdd-new-change]` | `resolve_optional_skills`（`templates.rs:61-74`）：过滤不匹配 `OPTIONAL_SKILL_FILES` 的项 | 静默忽略（不报错，不写入） |
| 新 `.agents/skills/llman-sdd-draft/` | `write_tool_skills`：候选集含 draft | 自动写入 |

**风险**：config 里失效的 extra_skills 条目被静默忽略，用户可能不知道。**缓解**：在 `update-skills` 成功输出里，若检测到 config 含已不在 optional 列表的条目，打印一行 WARNING 提示。但这属于增强，非本 change 必须——本 change 先依赖自愈，WARNING 增强可后续补。

## draft 技能模板职责边界

| 职责 | draft 技能 | propose 技能 |
|---|---|---|
| 仅创建 proposal.md（draft shell） | ✅ 唯一职责 | ❌（指向 draft） |
| triage（判断变更规模） | ❌ | ✅ |
| tasks / design / live specs / change start | ❌ | ✅ |
| 触发条件 | 用户说「draft/记一个/先把X记下来」 | 用户要求正式提案 |

## 测试边界（seam）

- **CLI 子进程**：`llman sdd update-skills --tool codex --skills-only` 后产物含 `.codex/skills/llman-sdd-draft/SKILL.md`，不含 `llman-sdd-new-change`。
- **静态**：`templates/sdd/{en,zh-Hans}/skills/llman-sdd-draft.md` 存在；`llman-sdd-new-change.md` 已删。
- **渲染**：propose 模板渲染产物不再含完整 draft 内联段（仅一句指向）。
- **Rust 常量**：`DEFAULT_SKILL_FILES` 含 `llman-sdd-draft.md`；`OPTIONAL_SKILL_FILES` / `OPTIONAL_SKILL_NAMES` 不含 new-change。

## 不做的事

- 不改 `change new --from` CLI（已完备）。
- 不新增 config 迁移代码（依赖自愈）。
- 不改 BDD runner 行为。
