# llman SDD 学习文档（agent skill 链路）

本目录描述 **agent 如何选择 skill**，以及 BDD-on / BDD-off 两条闭环。

| 文档 | 内容 |
|---|---|
| [pipeline-bdd-on.md](./pipeline-bdd-on.md) | BDD-on（`config.yaml` 含 `bdd:`）：Git-native Partitioned SSOT |
| [pipeline-bdd-off.md](./pipeline-bdd-off.md) | BDD-off（无 `bdd:`）：change 内 TOON delta → archive 合并 |

## 应急方案 vs 元 skill（方向）

**当前应急（change `update-skill-bdd-mode-conditioning`）**

- 同一套模板 + MiniJinja `{% if bdd_enabled %}` 条件渲染
- 产物带 `metadata.llman_sdd.bdd_mode` / `skill_set`
- `validate` / `init --update` / `update-skills` 不一致则 ERROR，并提示刷新

**后续方向（draft `add-meta-skill-dynamic-prompts`）**

- 项目内只留 bootstrap 元 skill
- 运行时由 `llman sdd` 按 stage / bdd / `extra_skills` 吐出当步指令
- 评估后再正式 propose，本目录 README 仅作路标

## 默认不安装全部 skill

`extra_skills` 默认关闭；optional（continue / ff / sync / …）需显式启用。
Agent 交叉引用应门控，避免推荐未安装 skill。
