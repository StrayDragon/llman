# Design: Skill BDD 条件渲染与一致性门禁

## D1. 模板策略：单源 + MiniJinja（B2）

不建 `skills/bdd-on/` 与 `skills/bdd-off/` 物理双目录（避免双份漂移）。

- 渲染变量沿用现有 `bdd_enabled`（`build_template_vars`）。
- 新增/强化变量：`extra_skills` 集合（或 `extra_skill_continue` 等布尔），供交叉引用门控。
- 条件块粒度：以「整段路径说明」为单位，避免每个命令一行 if。
- **保留** 各 skill 内 mermaid pipeline 图（注意力锚点）；可按模式略改节点文案。
- Token 优化（不砍 mermaid）：
  - `sdd-commands` unit 按 `bdd_enabled` 裁剪命令行（BDD-on 弱化 delta；BDD-off 弱化 finalize）。
  - `structured-protocol` 的空 Ethics 占位改为短约束句或按 skill 注入真实 ethics（避免无值列表）。

## D2. 元信息 schema（C1）

```yaml
metadata:
  version: "0.0.x"
  llman_sdd:
    bdd_mode: on | off
    skill_set: default | optional
```

- `bdd_mode`：`config.bdd.is_some() → on`，否则 `off`。
- `skill_set`：默认集 → `default`；`OPTIONAL_SKILL_FILES` 成员 → `optional`。
- 解析：宽松读 YAML frontmatter；缺 `llman_sdd` / 缺 `bdd_mode` → 视为不一致（ERROR）。

## D3. 检查挂载点（D1 高质量）

共享函数（建议 `src/sdd/project/skill_consistency.rs`）：

```text
check_installed_skills_bdd_mode(root, config) -> Result<(), SkillConsistencyError>
```

调用方：

| 入口 | 时机 |
|---|---|
| `validate`（all / specs / change / single spec） | 校验规格前或后，失败即整体失败 |
| `init --update` | 刷新 skills **之后** 再 check（保证新产物自洽）；若刷新失败则不掩盖 |
| `update-skills` | 写入完成后 check |

错误文案 MUST 含：期望 `bdd_mode`、至少一个违规 skill 路径、修复命令
`llman sdd init --update`（及 `update-skills` 别名说明）。

## D4. Context 懒刷新（E3）

在 `context_run_pageindex`：

```text
match freshness:
  Fresh -> retrieve
  Stale | Missing -> rebuild_pageindex; then retrieve
  Corrupted -> rebuild_pageindex; on rebuild fail -> error; else retrieve
```

- rebuild 仍持 `.rebuild.lock`。
- stderr/JSON：可在 `qualityNote` 标注 `auto-rebuilt: true`（可选，便于调试）。
- 不改变「无 chat model → api_error」语义。

## D5. 文档布局

```text
docs/sdd/
  README.md
  pipeline-bdd-on.md
  pipeline-bdd-off.md
```

图用 mermaid；与 skill 内图语义一致，供人类学习，不替代 skill。

## D6. 与 draft「元 skill」关系

本次是应急静态条件渲染。`add-meta-skill-dynamic-prompts` 评估：项目内只留一个
bootstrap skill，运行时由 `llman sdd` 按 stage/bdd/extra_skills 吐出指令。本次门禁与
元信息字段应为该方向预留兼容（`llman_sdd.*` 命名空间）。

## D7. 风险

- validate 每次扫 `.agents/skills`：目录很小，可接受；可缓存 mtime 但非必须。
- 旧项目首次升级必跑 `init --update`：升级指南 / 错误提示必须醒目。
