# llman SDD 学习文档（agent skill 链路）

本目录描述 **agent 如何选择 skill**，以及统一 Git-native 变更闭环。

| 文档 | 内容 |
|---|---|
| [pipeline-bdd-on.md](./pipeline-bdd-on.md) | 统一 Git-native 流程（`draft` → `designed` → `change start` → apply → verify → archive/finalize） |
| [pipeline-bdd-off.md](./pipeline-bdd-off.md) | **已废弃** — 见上方统一流程 |

## 统一流程要点

- 生命周期：`draft` → `designed` → **`change start`**（或 `attach`）→ apply → verify → archive/finalize
- Spec SSOT：在 feature 分支上编辑 live `llmanspec/specs/**`（**不要**在 `changes/<id>/specs/` 下写 delta）
- `bdd:` 段仅为 **runner 开关**（`validate --check` 是否执行 `bdd.run_command`），不分叉生命周期
- 归档：`change finalize` / `change archive` 先 **ff-merge** feature 到默认分支，再 **改名** change 文档到 `archive/`（脏改名留一次 commit）
- 已移除：`change delta`、`llman-sdd-sync`、solidify、TOON delta merge

## 应急方案 vs 元 skill（方向）

**当前应急（change `update-skill-bdd-mode-conditioning`）**

- 同一套模板 + MiniJinja `{% if bdd_enabled %}` 条件渲染（逐步收敛为统一文案）
- 产物带 `metadata.llman_sdd.bdd_mode` / `skill_set`
- `validate` / `init --update` 不一致则 ERROR，并提示刷新

**后续方向（draft `add-meta-skill-dynamic-prompts`）**

- 项目内只留 bootstrap 元 skill
- 运行时由 `llman sdd` 按 stage / bdd / `extra_skills` 吐出当步指令
- 评估后再正式 propose，本目录 README 仅作路标

## 默认不安装全部 skill

`extra_skills` 默认关闭；optional（continue / ff / …）需显式启用。
Agent 交叉引用应门控，避免推荐未安装 skill。
