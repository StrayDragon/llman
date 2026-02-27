## 1. 明确两类贡献者路径（解耦 prompt 工程师与项目开发者）

- [x] 1.1 新增文档 `docs/sdd-prompt-engineering.md`，明确：
  - Prompt/Context 工程师：主要修改 `templates/sdd/**` 与 `templates/sdd-legacy/**`，并运行门禁 + Arena 评估
  - 项目开发者：负责必要的 Rust/脚本/测试改动，降低 prompt 工程师迭代摩擦
  - 最小命令集（建议顺序）：
    - `export OPENAI_DEFAULT_MODEL=gpt-5.2`（评估默认模型；未设置时，入口脚本应报错或要求显式传参）
    - `just check-sdd-templates`
    - `cargo +nightly test --test sdd_integration_tests -q`（模板相关回归）
    - `llman sdd validate --ab-report --json --no-interactive`（new vs legacy 快速门禁）
    - `LLMAN_CONFIG_DIR=./artifacts/testing_config_home llman x arena ...`（深评估）

- [x] 1.2 为 prompt 工程师提供“单入口”执行方式（任选其一，并写进上面的文档）：
  - A) `just sdd-prompts-eval`（推荐）
  - B) `scripts/sdd-prompts-eval.sh`
  该入口 MUST 做到：不要求在 repo 根目录创建 `./llmanspec/` 也能跑通评估闭环。
  该入口 SHOULD 支持通过 `OPENAI_DEFAULT_MODEL` 选择模型；未设置时 MUST 报错，或要求通过命令行参数显式指定（避免意外跑到 contest 默认模型）。

## 2. 处置 `templates/sdd/**/skills/shared.md`（去繁就简，避免影子真源）

- [x] 2.1 确认 `templates/sdd/*/skills/shared.md` 及 legacy 对应文件不参与 `llman sdd update-skills` 生成链路（当前不在 `SKILL_FILES`）。
- [x] 2.2 在 new + legacy 双轨、`en` + `zh-Hans` 双语下执行同一策略（择一落地）：
  - 删除该文件（推荐，降低误导与维护成本）；或
  - 改为“指针页”（只保留链接/引用 units 的入口，不重复粘贴内容，并明确真源在 `templates/**/units/**`）。
- [x] 2.3 若删除/移动该文件，补齐必要的 Rust 侧调整（例如 embedded template mapping）并运行回归：
  - `just check-sdd-templates`
  - `cargo +nightly test --test sdd_integration_tests -q`

## 3. 建立可复现的 Arena 评估套件（repo-tracked fixtures）

- [x] 3.1 在 `artifacts/testing_config_home/arena/` 下新增：
  - `datasets/sdd_apply_v1.yaml`（聚焦 `apply` 的典型场景；均为 `type: text`）
  - `contests/sdd_apply_v1.toml`（至少包含两个 prompt variants：baseline 与 candidate）
  - （后续扩展）可按需新增 `sdd_new_v1`/`sdd_verify_v1`/`sdd_explore_v1`，但本轮先把 `apply` 的闭环跑通
- [x] 3.2 在 `artifacts/testing_config_home/prompt/<app>/` 下新增 prompt variants 的落地点（建议先用 `codex`）：
  - baseline：优化前快照（或 legacy 风格渲染产物）
  - candidate：优化后当前工作区渲染产物（或 new 风格渲染产物）
- [x] 3.3 为 prompt 工程师提供“自动生成/刷新 baseline 与 candidate prompts”的方式（脚本或 just 命令）：
  - 从当前模板渲染得到纯文本 prompt（必须展开 `unit()`，不得包含 `{{ unit(`）
  - 写入/刷新到指定 `LLMAN_CONFIG_DIR` 下的 `prompt/<app>/`（默认使用临时目录，避免污染仓库内 fixtures）
  - 允许选择 style（new/legacy）与 locale（en/zh-Hans）以便做针对性评估

## 4. 渐进式优化 SDD prompts（逐个入口迭代，双轨双语同步）

约束（每次只做 1–2 个入口，确保可回归）：
- 同步修改 new + legacy
- 同步修改 `en` + `zh-Hans` 且保持模板版本号一致
- 每轮至少跑：`just check-sdd-templates` + 目标相关回归测试

- [x] 4.1 优先优化 `apply`（高风险入口，最易暴露跑偏）：
  - `templates/sdd/*/spec-driven/apply.md`
  - `templates/sdd/*/skills/llman-sdd-apply.md`
  - 以及 legacy 对应文件
  - 目标：移除 “Options/<option>” 等占位、去除重复 guardrails、统一 STOP/澄清/校验表达

- [x] 4.2 优化 `new`：
  - `templates/sdd/*/spec-driven/new.md`
  - `templates/sdd/*/skills/llman-sdd-new-change.md`
  - 以及 legacy 对应文件

- [x] 4.3 优化 `verify`：
  - `templates/sdd/*/spec-driven/verify.md`
  - `templates/sdd/*/skills/llman-sdd-verify.md`
  - 以及 legacy 对应文件

- [x] 4.4 优化 `explore`：
  - `templates/sdd/*/spec-driven/explore.md`
  - `templates/sdd/*/skills/llman-sdd-explore.md`
  - 以及 legacy 对应文件

- [x] 4.5 收尾：其余入口（continue/ff/sync/archive/show/validate/specs-compact）按重复度与失败率排序逐个处理。

## 5. 回归门禁与验收记录

- [x] 5.1 添加/更新集成测试，锁定关键不变式（至少覆盖）：
  - 生成产物不包含 `Options:`、`<option`、`What would you like to do?`
  - 生成产物不包含未展开标记 `{{ unit(`
  - new 风格治理字段仍被强制（缺失则失败）
- [x] 5.2 固化评估闭环命令并记录一次结果（写入 `openspec/changes/optimize-sdd-prompts/implementation-notes.md`）：
  - `llman sdd validate --ab-report --json --no-interactive`
  - `OPENAI_API_KEY=... OPENAI_BASE_URL=...（或 OPENAI_API_BASE=...） LLMAN_CONFIG_DIR=./artifacts/testing_config_home llman x arena gen --contest sdd_apply_v1 --dataset sdd_apply_v1 --rounds <N> --seed <S>`
  - `LLMAN_CONFIG_DIR=./artifacts/testing_config_home llman x arena vote --run <RUN_ID>`
  - `LLMAN_CONFIG_DIR=./artifacts/testing_config_home llman x arena report --run <RUN_ID>`
