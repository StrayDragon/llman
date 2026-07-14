# Proposal: Feature-as-Spec — 目录即规格

## Why

当前 spec 用人类 prose 描述行为合约（`requirements[].statement`），有三个死穴：

1. **漂移**：代码改了 prose 不更新，`validate` 仍通过。
2. **不可执行**：`"System MUST do X"` 无法自动验证真伪。
3. **Token 浪费**：agent 每次读 spec 要消化长篇 prose（~75 tokens / 需求）。

已有但未被采用的基础设施：`feature_refs` IR 字段、`validate_feature_refs`（解析 `.feature`）、`gherkin` crate、`BddConfig`。经核查（`grep -rln feature_refs llmanspec/specs/` 为空、`find . -name "*.feature"` 为空），**这些是零用户的死代码**。

本 change 的北极星：**让 `.feature` 文件成为 spec 本身，prose 是过渡形态，最终被 `.feature` 替代。**

## What Changes（一句话）

> BDD-on 模式下，`llmanspec/specs/<name>/` 目录里的 `.feature` 文件**就是** spec；`spec.toon` 瘦身为 capability 元数据卡片；`requirements[].statement` 渐进迁移为同目录的 `.feature`。

### 1. 目录即 spec（砍 indirection）

```
llmanspec/specs/cli/
├── spec.toon          ← 瘦身到元数据（见下）
├── status.feature
├── priority-sort.feature
└── json-output.feature
```

新增一个 `.feature` = 丢个文件进目录，**无需注册、无漂移**。llman 用 `glob specs/<name>/*.feature` 自动发现。

**为何不要 `evidence[]` 表 / `verify.yaml` manifest**：directory 即分组，表只是重复列目录内容，而重复 = 漂移温床。indirection 当初挣到位置靠"混合工件类型"和"引用测试文件"两个需求，但本 change 已消解它们——`.feature` 是唯一规格，迁移是 prose→feature 直跳。需求没了，indirection 砍掉。

### 2. spec.toon 终态瘦身

```toon
kind: llman.sdd.spec
name: cli
purpose: "CLI command definitions and argument parsing."
```

`requirements[]` / `scenarios[]` 在迁移期保留（BDD-on 模式下逐条迁出），终态消失。**`valid_scope` 在 BDD-on 模式禁用**——`.feature` 文件位置就是 scope 真相，hint 会漂移。`purpose` 保留（capability 级元数据，无处可去）。

### 3. 仅 BDD-on 生效（干净并存）

`config.yaml` 的 `bdd:` 段是开关。复用现有 `bdd_enabled` 门控（`validation.rs:759`）：

| 模式 | 触发 | spec 形态 |
|------|------|-----------|
| BDD off | 无 `bdd:` 段 | 现 prose world 完全不动 |
| BDD on | 有 `bdd:` 段 | 目录即 spec，`.feature` 是规格，`requirements[]` 渐进迁出 |

**不强制数据迁移**。旧项目不开 `bdd:` 就继续用 prose。

### 4. 两种校验 mode

| mode | 命令 | 作用 |
|------|------|------|
| **fast**（默认） | `llman sdd validate <spec>` | glob `.feature` + gherkin 语法解析（复用 `gherkin::Feature::parse`）+ 存在性。瞬时、零执行、CI 廉价 = **漂移守卫** |
| **full** | `llman sdd validate <spec> --check` | 再跑 `bdd.run_command` / `default_check`（项目级配置一次）执行所有 features，汇总 pass/fail |

fast 保证"规格结构在且合法"；full 保证"实现真的满足规格"。双层护城河。

### 5. Locale 感知

`.feature` 支持中文 Gherkin 关键字（`功能`/`假如`/`当`/`那么`/`而且`）。`lang` 由 `config.yaml` 的 `locale` 推导，传给已有的 `gherkin::GherkinEnv::new(lang)`（`validation.rs:665`）。parser 侧零改动；真正的工作在 skills 约定（propose/apply/verify prompt 指导 agent 按 locale 写关键字）。

### 6. 砍死代码

删除零用户的 BDD 基础设施，净简化：
- `feature_refs` IR 字段（`ir.rs:18-20`）+ `FeatureRefEntry` 结构体
- `validate_feature_refs`（`validation.rs:614-728`）→ 逻辑迁移进新的 directory-based validator
- `point-only` guardrail（`validation.rs:745-777`）→ 被"目录即 spec"取代

### 7. Scope integrity hook 退休（BDD-on 模式）

commit `9ba8da0` 的 scope integrity hook 依赖 `valid_scope`。BDD-on 模式禁用 `valid_scope` 时，hook 在该模式退休——"feature 属于 spec A" = "文件在 `specs/A/`"，目录结构是 SSOT，不漂移。BDD-off 模式保留（旧 world 不动）。

## Capabilities

| Capability | Change | Type |
|------------|--------|------|
| `sdd-workflow` | 新增 r51–r55：feature-as-spec 模式、fast/full validate、locale 感知、BDD-on 门控、valid_scope 退休 | add |
| `sdd-legacy-compat` | 新增 r2：feature_refs 删除，无自动迁移路径 | add |

## Impact

- **Token**：终态 spec ~15 tokens vs 当前 prose ~525 tokens/spec（11 reqs × ~75）。agent 读 spec 成本正比于单个 capability，不随项目规模膨胀。
- **SSOT**：`.feature` 文件是唯一真相；目录结构即分组；无 manifest 注册表漂移。
- **Breakage**：无强制迁移。BDD-off 模式行为不变。开 `bdd:` 段才进入新 world。
- **死代码清理**：删除 `feature_refs` / `validate_feature_refs` / point-only guardrail（均为零用户）。

## Migration Path

不提供自动迁移命令（符合 r41 "无自动迁移" 原则）。迁移是人工作业：开 `bdd:` 段 → 逐条把 `requirements[].statement` 提炼成同目录 `.feature` → 删掉该 statement。每迁一条，fast mode 守住结构合法性，full mode 守住实现满足。

## 非目标（显式排除）

- **llman 不当 test runner**：full mode 只 shell-out 用户配置的命令（`bdd.run_command`），不持有任何 framework 知识、不做 per-type dispatch。
- **无工件类型体系**：无 `kind`/`type` 字段驱动验证逻辑。`.feature` 是唯一规格工件；测试文件是实现，不被 spec 引用。
- **无 evidence[] 表 / verify manifest**：目录即 spec，无需注册。
- **无跨 capability 全局 registry**：token locality 正比于单个 spec，不随项目膨胀。
