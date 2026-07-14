# Design: Feature-as-Spec — 目录即规格

## 核心模型

```
BDD-on 模式（config.yaml 有 bdd: 段）：

spec = 目录里所有 *.feature 文件
       + spec.toon（仅 kind/name/purpose 元数据卡片）

BDD-off 模式（config.yaml 无 bdd: 段）：

spec = 现有 prose world（requirements[] + scenarios[]），完全不动
```

**没有中间层**：无 `evidence[]` 表、无 `verify.yaml` manifest、无 artifact registry。目录结构即分组，文件存在即注册。

## 目录结构

```
llmanspec/specs/cli/
├── spec.toon              ← 元数据卡片（BDD-on 下瘦身）
├── status-output.feature  ← 行为规格（Gherkin）
├── priority-sort.feature
└── json-compat.feature
```

- 新增 feature = 丢文件进目录。无需任何注册步骤。
- 删除 feature = 删文件。
- 跨 capability 共享：同一 `.feature` 路径不会出现在两个目录里（物理约束）。

## spec.toon 形态对比

### 当前（BDD-off，保持不变）

```toon
kind: llman.sdd.spec
name: cli
purpose: "CLI command definitions and argument parsing."
valid_scope[3]: src/sdd/command.rs,src/cli.rs,"llmanspec/specs/cli/"
requirements[11]{req_id,title,statement}:
  r1,"Config dir guard","System MUST only perform dev-project..."
  r2,Status Command,"System MUST provide a `status` subcommand..."
  ...
scenarios[18]{req_id,id,given,when,then}:
  ...
```

### 迁移中（BDD-on，requirements 逐条迁出）

```toon
kind: llman.sdd.spec
name: cli
purpose: "CLI command definitions and argument parsing."
requirements[2]{req_id,title,statement}:    ← 还剩 2 条未迁
  r9,TOON Output Format,"TOON output MUST use kind `llman.sdd.status`..."
  r10,Priority Sort,"Output MUST be sorted by priority..."
# (r1..r8 已迁为同目录 .feature)
```

注意：BDD-on 模式下 `valid_scope` 被忽略（`.feature` 位置是 scope 真相）。

### 终态（BDD-on，全部迁完）

```toon
kind: llman.sdd.spec
name: cli
purpose: "CLI command definitions and argument parsing."
```

3 行。行为规格全在同目录 `.feature` 文件里。

## 校验流水线

```
llman sdd validate <spec>            ← fast mode（默认）
  │
  ├── 读 config.yaml → bdd_enabled?
  │     ├── false → 走现有 prose 校验路径（不变）
  │     └── true  → feature-as-spec 路径 ↓
  │
  ├── 1. glob llmanspec/specs/<name>/*.feature
  │     ├── 0 个 feature + requirements 非空 → OK（迁移中）
  │     ├── 0 个 feature + requirements 空   → ERROR（空 spec）
  │     └── N 个 feature → 逐个解析 ↓
  │
  ├── 2. 对每个 .feature:
  │     └── gherkin::Feature::parse(content, GherkinEnv::new(lang))
  │           ├── lang 来自 config.yaml locale 推导（zh-Hans → zh-CN）
  │           ├── 解析失败 → ERROR（结构非法，给修复提示）
  │           └── 解析成功 → OK（结构合法）
  │
  └── 3. 输出: N features parsed, M requirements pending migration


llman sdd validate <spec> --check    ← full mode
  │
  ├── 先跑完 fast mode 全部步骤
  │
  ├── 4. 读 bdd.run_command（或 framework 默认）
  │     └── 例: "cucumber-rs {feature_dir}" / "pytest {feature_dir}"
  │
  ├── 5. shell-out 执行（整个 spec 目录一次命令，不逐 feature）
  │     ├── 退出码 0 → 所有 feature pass
  │     └── 退出码 != 0 → 报告 fail（读 runner 输出，v1 不做结构化映射）
  │
  └── 6. 汇总: X passed / Y failed
```

**关键：full mode 跑一次命令，不是逐 feature**。所有 BDD runner 都是 batch-discovery（cucumber/pytest 发现全部 `.feature`），告诉 runner 目录即可。

## BDD-on 门控（复用现有 bdd_enabled）

现有代码已有 `bdd_enabled = bdd_config.is_some()`（`validation.rs:83`）和 point-only 逻辑（`validation.rs:745-777`）。本 change：

1. 保留 `bdd_enabled` 判定（config.yaml 有 `bdd:` 段即 true）
2. **替换** point-only guardrail 逻辑：BDD-on 时改为 directory-based feature 发现（取代 `feature_refs` 指针）
3. **BDD-on 模式忽略 `valid_scope`**（不在校验中要求它，staleness check 在该模式跳过 scope 匹配）

## Locale → Gherkin lang 映射

`config.yaml` 的 `locale` 已有值（`zh-Hans` / `en`）。推导 Gherkin lang：

| locale | gherkin lang | 关键字示例 |
|--------|-------------|-----------|
| `en` | `en` | Feature / Given / When / Then |
| `zh-Hans` | `zh-CN` | 功能 / 假如 / 当 / 那么 |

映射逻辑：`zh-Hans*` → `zh-CN`，其余透传。传给 `gherkin::GherkinEnv::new(lang)`（已存在于 `validation.rs:665`）。

## config.yaml 扩展

`BddConfig` 已有字段基本够用，仅微调语义：

```yaml
bdd:
  framework: cucumber-rs          # 已有
  feature_dir: llmanspec/specs    # 已有；feature-as-spec 下指向 specs 根
  default_language: zh-CN         # 已有；优先于 locale 推导
  run_command: "cargo test --features bdd"  # 已有；full mode 执行用
  # verify_prompt 保留但降级（BDD-on 下 verify skill 读 features 而非 prompt）
```

**不新增字段**。`feature_dir` 语义从"独立 feature 根目录"变为"per-capability specs 目录的父级"。

## 死代码清理清单

| 删除项 | 位置 | 原因 |
|--------|------|------|
| `feature_refs` 字段 | `ir.rs:18-20` | 0 用户，被目录发现取代 |
| `FeatureRefEntry` 结构体 | `ir.rs:23-29` | 同上 |
| `validate_feature_refs` | `validation.rs:614-728` | 逻辑迁移进新 validator |
| `FeatureRef` presentation wrapper | `parser.rs:21-36` | 随字段删除 |
| point-only guardrail | `validation.rs:745-777`（部分） | 被 directory-based 逻辑取代 |

注意：`gherkin` crate（`Cargo.toml:85`）**保留**——新的 feature validator 复用它。

## Scope Integrity Hook 处理

commit `9ba8da0` 的 scope hook 依赖 `valid_scope` 做 staleness 校验（sdd-workflow r15）。

- **BDD-off**：hook 行为不变（用 `valid_scope` 匹配 git diff）。
- **BDD-on**：hook 在 spec 级 staleness 跳过 scope 匹配——feature 文件位置是结构性的 scope 真相，不需 hint。改为"该 spec 目录下任何文件变更 = 该 spec 被触及"。

## Skills 调整（约定层，非代码）

| Skill | BDD-on 调整 |
|-------|-------------|
| `llman-sdd-propose` | 识别涉及行为 → 在目标 spec 目录创建 `.feature`（按 locale 写关键字）→ 从 requirements 删除对应 statement |
| `llman-sdd-apply` | 读 spec 目录 `.feature` → 实现使 step definitions 通过 → `validate <spec> --check` 验证 |
| `llman-sdd-verify` | fast: `validate <spec>`（结构合法）；full: `validate <spec> --check`（实现满足）。对比代码与 features |
| `llman-sdd-archive` | 合并 delta specs；`.feature` 文件随 spec 目录一起归档/合并 |

这些是 prompt 约定，通过 `templates/sdd/<locale>/skills/*.md` 更新，不改 CLI 逻辑。

## 实现顺序

### P0：核心数据模型 + fast mode
- 删除 `feature_refs` IR 字段 + `FeatureRefEntry` + `FeatureRef` wrapper
- 实现 directory-based feature discovery（`glob specs/<name>/*.feature`）
- 实现新 `validate_features_dir()`：gherkin 语法解析 + locale lang 推导
- BDD-on 门控：`bdd_enabled` → 走新路径；BDD-off → 走旧路径
- BDD-on 模式忽略 `valid_scope` + scope hook 退休

### P1：full mode
- `validate --check` flag
- 读 `bdd.run_command` / `effective_run_command()`
- shell-out 执行 + 退出码汇总

### P2：Skills 更新
- propose/apply/verify/archive skills prompt 增加 BDD-on 分支
- locale-aware Gherkin 关键字指导

## 已知限制（标 P2/已知限制，非本 change 阻塞项）

### 限制 1：openspec interop 导出丢失 .feature 语义

`src/sdd/project/interop.rs` 的 export 只解析 spec.toon（`feature_refs: None` at line 332）。BDD-on 项目的 `.feature` 文件若被目录级复制到 openspec 侧，openspec 没有对应概念——导出的 `openspec/` 会丢失 .feature 的结构化行为规格。

**处理**：openspec interop 是独立 capability（`sdd-openspec-interop`），openspec 本身用 prose scenarios。本 change 不扩展 interop。若用户从 BDD-on 项目 export 到 openspec，.feature 内容不随迁（已知限制，文档标注）。未来可加 `export --style openspec` 的 BDD-aware 分支作为独立 draft。

### 限制 2：openspec import 补的 valid_scope 在 BDD-on 被忽略

`sdd-openspec-interop` r9 要求 import 时补齐 `llman_spec_valid_scope` frontmatter。但本 change r51/r55 在 BDD-on 模式禁用 valid_scope。

**交互**：import 到一个 BDD-off 项目 → valid_scope 正常补齐和使用（无冲突）。import 后用户手动开 `bdd:` 段 → valid_scope 被忽略（r55），frontmatter 里的值无害但冗余。

**处理**：无代码改动。import 补的 valid_scope 在 BDD-off 仍有效；BDD-on 下静默忽略。低优先级，因为触发路径是「import 后再手动开 BDD」。

## 开放问题（留给后续 draft）

1. **批量管理命令**（`llman sdd spec scan-features` / bulk migrate prose→feature）——独立后续 draft，非本 change 范围。
2. **full mode 输出结构化映射**（runner 输出 → 哪个 feature pass/fail）——v1 人工读输出，P2 再做解析。
3. **非 BDD-on 项目想引用测试文件作证据**——显式排除（测试是实现，不是规格；要精确规格就开 BDD-on 写 `.feature`）。
