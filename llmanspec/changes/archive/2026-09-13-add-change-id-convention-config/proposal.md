---
depends_on: []
branch: sdd/add-change-id-convention-config
base_sha: f959b04981fc88347bb34fc8431a88f837a2f1f5
base_branch: main
---

## Why

change id 命名规约（如下游仓库 xylitol 的 `c{priority}-{verb}-{subject}`，写在项目 `llmanspec/AGENTS.md` 散文里）目前工具链完全无法执行：

- `change new --from` 的 id 派生是纯启发式消毒（`crates/llman-sdd/src/sdd/change/new.rs` 的 `derive_change_id`：小写、非 ASCII 字母数字丢弃、60 字符截断），doc 注释明确「intentionally heuristic ... does **not** impose a fixed naming convention」，规约遵循被委托给读 AGENTS.md 的 agent/人——靠自觉。
- `llman sdd validate` 的 `validate_sdd_id`（`sdd/shared/ids.rs`）只查非空/非路径形，不查格式。
- `--from` help 文本宣称 *"Follows naming conventions declared in the repo's `llmanspec/AGENTS.md`"*，但实现并不读该文件——文档承诺与实现有出入。
- 取号撞号（issue #20 实测，xylitol 2026-09-13）：只扫 active `changes/` 与 `changes/archive/` 首层得出「最大号 c25」→ 误取 `c10`；全树核查后真实最大为 c2790（`delayed-changes/` 递归含 c2620、`changes/archive/freezed_changes.7z.archived` 冻结包含 c2790）。现有冲突检查只查目录名精确相等（`proposal_path.exists()`），同号不同后缀的 id（`c10` vs `2026-09-13-c20-…` 形态）无法被发现。

## What Changes

`llmanspec/config.yaml` 新增**可选** `change_id:` 段（与 `archive:` / `bdd:` / `sdd:` 平级），覆盖「校验」「生成」「预览」三端；未配置时所有行为与现状完全一致（零破坏）：

1. **validate 门禁 `pattern`（用户配置正则）**：`llman sdd validate` 对 active change id 做 full-match，违规 = ERROR 条目（含 change id 与 pattern 原文）。校验范围只约束 active `changes/`（发现产物），archive/存量形态天然不回溯强制。
2. **生成模板 `template` + 预设变量（minijinja，已在 workspace 依赖）**：`change new --from` 渲染模板生成 id。预设变量：`llman_sdd_unique_id`（全树查重后的下一个未占用号）、`verb`（动词归一或 `--verb` 显式指定）、`subject`（现 `derive_change_id` 的 kebab 产物）、`date`（本地日期）。新增 `--dry-run`（或 `--print-id-only`）只渲染不落盘。**取号查重范围 = `llmanspec/` 全树递归**（`changes/`、`changes/archive/`、`delayed-changes/` 等任意层级子目录中 change-id 形态的目录名）；人工冻结包（`*.7z.archived` 等）best-effort 扫描（系统有 `7z` 时），不可用时 WARNING 提示人工核对。查重全树与 pattern 校验只管 active 是两个不同范围。
3. **取号预览子命令 `llman sdd change next-id`**：只读，打印当前全树最大号与下一个可用号（即 `llman_sdd_unique_id` 的取值依据），人类不建目录即可核对取号。
4. **help 文本诚实化**：`--from` 的 help 措辞与实现对齐（配置 template 后「follows naming conventions」由模板保证；未配置时措辞改为描述启发式消毒现状）。
5. **规约变更**：`sdd-workflow` 新增 `@req:r29 @human`（三端能力合约）+ `@executable` 验收场景；`sdd-structured-skill-prompts` r99 追加「配置 `change_id.template` 时 MUST 按模板渲染」语义（lock 报告制 WARNING，允许）。

实现零新增依赖：minijinja 2.20.0 与 regex 均已在 workspace 依赖（llman-sdd 已用 minijinja 渲染 skill 模板）；`ChangeIdConfig` 走 schemars derive 自动进 config schema（`llman self schema` 闭环既有）。

## Capabilities

- `sdd-workflow`（新增 r29：change id 命名规约机读化；pattern 门禁 + 模板生成 + next-id 预览）
- `sdd-structured-skill-prompts`（r99 描述扩展：template 配置下的生成语义）
- `config-schemas`（新段由既有 schema 生成/校验闭环自动覆盖，规约文本无需修改）

## Impact

- 代码：`crates/llman-sdd/src/sdd/project/config.rs`（新段 struct）、`sdd/change/new.rs`（模板渲染 + dry-run）、`sdd/change/mod.rs` + `sdd/command.rs`（next-id 子命令）、`sdd/commands/validate.rs`（pattern 门禁）、新增 `sdd/shared/`（或 `change/`）全树扫描取号 helper。
- config schema：`artifacts/schema/configs/en/llmanspec-config.schema.json` 随构建刷新。
- 测试：`tests/it/` 集成模块 + `tests/bdd_steps.rs` 通用 step 驱动新增 `@executable` 场景。
- 向后兼容：`change_id:` 缺省 = 现状零行为变化；无破坏性字段移除，不需要 migrations 目录。
