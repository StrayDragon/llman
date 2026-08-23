---
depends_on: []
---

# list/show harness 计数拆分：bound 可声明口径

## Why

`llman sdd list --specs` 的 `harness` 列口径是「该 capability 的 `.feature` 文件里有几个场景」——只要写在 `.feature` 里就算数。Partitioned SSOT 下，场景只有被消费方绑定（llman 本仓：`tests/bdd_steps.rs` 的 `scenarios!("llmanspec/specs", tags = "@executable")` 目录发现；下游如 xylitol：逐场景 `#[scenario(path=…, name=…)]` 宏）才会真正执行。llman 不知道绑定关系，一律计入，导致：

- 本仓实测 28 个 capability 共 ~497 个 harness 场景，实际带 `@executable` 标签（即真正会被 `scenarios!` 展开为测试）的约 50 个。看列表的人会以为有近 500 个 BDD 场景在跑。
- 下游靠删 `.feature`（把内容迁进 toon 文档行）让数字「碰巧对」，列本身的口径缺陷还在。

探索阶段的反证（../xylitol，只读调查）：xylitol 的 `.feature` 零标签、全部经 `#[scenario(path,name)]` 绑定——若 llman 采用「按 `@executable` 标签推导 bound」的固定默认口径，会把 xylitel 从「碰巧对」变成「系统性错」（显示 bound=0/unbound=293，真相约 313/0）。**不存在对所有项目都正确的零配置默认口径**：绑定真相在消费方代码里。

因此本 change 把 bound 口径做成**可声明的**：项目在 `llmanspec/config.yaml` 声明绑定源；未声明时保持现口径，零回归。

## What Changes

1. **配置词汇表**：llmanspec 配置新增 `bdd.bindings` 列表，每项声明一个绑定源：
   - `kind: tags` —— 场景 tags 含所列标签即视为 bound（覆盖本仓 `scenarios!` + `@executable` 约定）；
   - `kind: scenario-attrs` —— 对声明 glob 匹配的文件提取 `#[scenario(path = "…", name = "…")]` 字面量对（覆盖 xylitol 类 per-scenario 形态）。
2. **输出拆分**（声明至少一个源时）：
   - `list --specs` 文本行由 `harness N` 拆为 `harness-bound B harness-unbound U`（B+U=N）；
   - JSON morphology 新增 `harnessBoundCount` / `harnessUnboundCount`；
   - `show <spec>` 的 Morphology 行同步同口径。
   未声明任何源时输出形态与今天完全一致（不出现误导性零值）；JSON 中两新键恒存在、未声明时为 `null`（沿用 r39 中 health/staleness 可为 null 的先例）。
3. **附带文档修正**：根 AGENTS.md 中「在 tests/bdd_steps.rs 用 `#[scenario(path=…, name=…)]` 绑定」的描述已过时（现实是 `scenarios!` 目录发现 + tag 表达式），顺带更正。

## Capabilities

- `sdd-workflow`（r39 morphology 字段合约修订 + 新增 bindings 声明 / 拆分两条 requirement；对应 feature 场景）

## Impact

- `src/sdd/project/config.rs`（`BddConfig` 加 bindings 段）
- `artifacts/schema/configs/en/llmanspec-config.schema.json`（schema 随类型再生成，遵循 config-schemas r49/r73 既有约束）
- 绑定解析新模块（tags 匹配 + scenario-attrs 提取）
- `src/sdd/spec/partitioned.rs`（`Morphology` 加字段）、`src/sdd/shared/list.rs`、`src/sdd/shared/show.rs`
- `AGENTS.md`（过时描述更正）
- 测试：单元（config 解析 / 绑定解析 / 拆分计算）+ `tests/sdd_bdd_compat_tests.rs`（morphology serde 兼容）

## Non-goals

- 不统计 capability 目录之外的 `.feature`（如 xylitol 的 `tests/features/cli-entry.feature`，20 个绑定落在 SSOT 外）——独立口径问题，另立 change。
- 不做绑定的隐式探测/兜底猜测（regex 启发式扫描全仓等），显式声明优先。
- 不要求 rstest-bdd 上游导出编译期 manifest（外部依赖，后续可作 scenario-attrs 的替代源再评估）。
- 不改变 validate / apply 门禁语义；`harness_scenario_count` 原键保留。

## Open Questions

（探索阶段已决，无遗留。设计取舍见 design.md：口径候选对比、默认行为、JSON nullability、scenario-attrs 提取规则的表达力边界、场景执行模式。）
