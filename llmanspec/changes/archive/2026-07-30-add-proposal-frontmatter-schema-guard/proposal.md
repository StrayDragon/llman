---
depends_on: []
branch: sdd/add-proposal-frontmatter-schema-guard
base_sha: da7c8179667ddbddb70436b02648e739024dd290
checkpointed: true
checkpoint_sha: da7c8179667ddbddb70436b02648e739024dd290
---

# 为 proposal frontmatter 增加 schema 守卫（未知字段报 ERROR）

## Why

`status: purpose-draft` 等 frontmatter 字段此前无成文约束，CLI 骨架不生成、`check_proposal_frontmatter` 不校验未知键、AGENTS.md 不定义合法字段集。结果是某次 agent 自发发明 `status` 字段后，靠范例模仿扩散成伪惯例，并进一步在正文复读（`> 草案（purpose-draft）` 横幅 / `## Status` 段）造成 SSOT 失效。

约定层（`llmanspec/AGENTS.md` 的「Change Proposal Frontmatter SSOT」）已声明 frontmatter 是元信息唯一权威，但**缺 CLI 机制背书**：没有校验堵住 agent 继续发明未知字段。本 change 补上机制层守卫，让伪惯例从源头不可存活。

## What Changes

- **定义合法 frontmatter 字段集**：`depends_on` / `blocks` / `branch` / `base_sha`（含 camelCase 别名 `baseSha`）/ `checkpointed` / `checkpoint_sha`（含别名 `checkpointSha`）。这些是 `check_proposal_frontmatter` 当前已识别的全部键。
- **`validate` 对未知字段报 ERROR**：active change 的 proposal frontmatter 出现合法集外的键（如 `status` / `title` / `priority` / `author`）时，`validate` 报 ERROR，错误消息列出合法字段集，引导清理。
- **stage 仍是推断量**：`determine_stage` 行为不变，继续从磁盘 artifacts + attach binding 推断 Draft/Designed/Full；MUST NOT 引入任何 frontmatter 字段影响 stage（`status` 永不参与）。
- **archived changes 免检**：`changes/archive/` 下的 proposal 不受未知字段校验约束（历史归档保持只读原样，零迁移成本）。
- **修正 AGENTS.md**：将刚提交约定中的「可选 status」改为「status 已废弃；生命周期阶段用 `llman sdd status` / `llman sdd show` 查看推断的 stage（r93 三态）」。

## Capabilities

- `sdd-workflow`（validate 对 proposal frontmatter 的未知字段守卫，新增 req）

## Impact

- **行为合约变更**：`validate` 对原本静默忽略的未知字段改为报 ERROR。已 attach 的 active change 若含历史遗留伪字段会首次报错——预期范围内，清理即可。
- **向后兼容**：archived 免检，无需迁移历史归档。
- **无破坏性字段移除**：不删除任何现有合法字段的读取，只新增未知键检测。
