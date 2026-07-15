# Design

## 1. 结构等价：TOON scenario ≡ Gherkin scenario

```
TOON                                     Gherkin

op_scenarios[1]{req_id,id,given,when,then}:
  r1,error-rendering,                  ↔  场景: error-rendering
  "llman 二进制已构建",                    假如 llman 二进制已构建
  "在非交互终端运行 llman sdd show",       当 在非交互终端运行 llman sdd show
  "退出码为 1 且 stderr 含 Error"          那么 退出码为 1
                                           那么 stderr 包含 Error
```

同一份 given-when-then 三元组的两种序列化格式。TOON 是 SSOT。

## 2. Solidify 流程

```
solidify <change-id> [--dry-run]
```

对 change delta 中每个 op_scenario 判断是否写入 `.feature`：

```
1. scenario 的 feature 字段显式为 false？
   → SKIP: "feature=false — 留在 toon"

2. when 文本自指？（见 §3）
   → SKIP: "recursive — 留在 toon"

3. 否则
   → WRITE to .feature
```

**框架无关**：solidify 不扫描 `tests/bdd_steps.rs`、不解析任何 BDD 框架的 step pattern。是否「可执行」由 `bdd.run_command` 在运行时判定——这是 `update-validate-bdd-auto-check` 已建立的职责分立。

## 3. 递归防护：自指检测

当一个 scenario 的 `when` 步骤调用 `llman` 自身，写入 `.feature` 会导致 BDD runner 在测试 validate 行为时 spawn 子 `cargo test`（即使有 `LLMAN_SDD_NO_BDD_CHECK` 深度断环，仍会增加不必要的一层嵌套编译运行）。

**检测规则**：`when` 文本匹配以下模式 → 跳过。

```
when 文本含 "llman sdd validate"
      或 "llman sdd archive"
      或 "llman sdd solidify"
```

注意：用 `llman sdd <subcmd>` 匹配而非裸 `validate`，避免误伤用户项目自身的命令名。

**替代方案已考虑**：

| 方案 | 问题 |
|------|------|
| 纯靠 ENV guard (`LLMAN_SDD_NO_BDD_CHECK`) | 不防无限（depth 1 仍 spawn cargo test → 编译慢 → 干扰 BDD 测试结果噪音） |
| 配置化黑名单 (`bdd.self_referencing_subcommands`) | 过度设计；只有三个 llman sdd 子命令有此问题 |
| 进程树追踪 | 过于复杂 |

## 4. 场景写入 feature 的显式控制

TOON scenario 新增可选字段 `feature`（默认 `true`）：

```toon
# 写法 1: feature=true（默认，可省略）
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,s1,"...","...","...",true

# 写法 2: feature=false — 明确留在 toon，不写入 .feature
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,s2,"...","...","...",false
```

`ScenarioEntry` 结构体新字段：

```rust
pub struct ScenarioEntry {
    pub req_id: String,
    pub id: String,
    pub given: String,
    #[serde(rename = "when")]
    pub when_: String,
    #[serde(rename = "then")]
    pub then_: String,
    #[serde(default = "default_feature_true")]
    pub feature: bool,
}

fn default_feature_true() -> bool { true }
```

**示例场景分类**：

| scenario | given/when/then | feature? | 走向 |
|----------|----------------|----------|------|
| error-rendering | `llman 二进制已构建` / `在非交互终端运行 llman sdd show` / `退出码为 1` | true (默认) | `.feature` ✅ |
| validate-self | ... / `运行 llman sdd validate errors-exit` / ... | false (显式) | `.toon` only |
| internal-flow | ... / `管理器扫描所有 spec 目录` / ... | false (显式) | `.toon` only |

## 5. spec.toon 结构变更

### Before (BDD-on)
```toon
kind: llman.sdd.spec
name: "errors-exit"
purpose: "..."
```

### After
```toon
kind: llman.sdd.spec
name: "errors-exit"
purpose: "..."
valid_scope[1]: llmanspec/specs/errors-exit
requirements[1]{req_id,title,statement}:
  r1,错误渲染,"System MUST render errors to stderr..."
scenarios[2]{req_id,id,given,when,then,feature}:
  r1,error-rendering,"...","...","...",true
  r1,internal,"...","管理器扫描...","...",false
```

### 校验变更

| 函数 | Before | After |
|------|--------|-------|
| `validate_spec_content_with_frontmatter_and_bdd` | BDD-on 时跳过 `valid_scope`、`requirements` 可为空 | 统一校验：valid_scope 必填、requirements 必含 MUST/SHALL、每个 req 至少 1 个 scenario |
| `validate_main_spec_doc` | BDD-on + 空 requirements → Info (feature-as-spec mode) | 删除分支。空 requirements → 永远 ERROR |
| `spec_dir_as_scope` (validate.rs) | BDD-on 时构造 scope from dir | 删除——`spec.toon` 的 `valid_scope` 是唯一来源 |

## 6. Archive 变更

### 删除的代码

```
src/sdd/change/archive.rs:
  struct FeatureUpdate          ← DELETE
  fn find_feature_updates()     ← DELETE
  fn copy_feature_files()       ← DELETE
  fn print_dry_run_features()   ← DELETE
  调用点 (run_with_root L121-127) ← DELETE
  测试 (L976-1054)               ← DELETE
```

### 保留不变

`find_spec_updates()` → `build_updated_spec()` → `write_updates()` 路径不改动。

## 7. 迁移工具

```
llman sdd project solidify-migrate [--dry-run]
```

扫描 `llmanspec/specs/*/`：
- 对每个 BDD-on spec（`spec.toon` 无 `requirements` 但有 `.feature` 文件）：
  - 从 `.feature` 反向提取 scenarios → 写入 `spec.toon`（所有 scenario `feature=true`）
  - 从 `.feature` 注释头部提取 purpose hint（如有）
  - `valid_scope` 设为 `llmanspec/specs/<id>`
- non-dry-run 时覆盖写文件

## 8. 受影响的文件清单

| 文件 | 变更 |
|------|------|
| `src/sdd/command.rs` | 新增 `Solidify` 子命令 + `Project SolidifyMigrate` |
| `src/sdd/spec/ir.rs` | `ScenarioEntry` 新增 `feature: bool` 字段 |
| `src/sdd/spec/validation.rs` | 移除 BDD-on valid_scope 豁免、空 requirements 特判 |
| `src/sdd/shared/validate.rs` | 移除 `spec_dir_as_scope()` |
| `src/sdd/change/archive.rs` | 删除 `FeatureUpdate` 相关全部代码 |
| `src/sdd/solidify.rs` | **新文件**：solidify 核心逻辑 |
| `src/sdd/project/solidify_migrate.rs` | **新文件**：迁移工具 |
| `locales/app.yml` | 移除废弃 i18n key |
| `.agents/skills/llman-sdd-solidify/SKILL.md` | **新文件** |
| `.agents/skills/llman-sdd-{propose,archive,specs-compact,graph}/SKILL.md` | 移除 `feature_refs` 引用 |
