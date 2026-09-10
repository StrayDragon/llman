# Design：Specs 布局扁平化 + 平铺迁移 + 协作提示

本文件是 `specs-flat-layout` change 的**自包含设计**：实现者只需读本文件 + `proposal.md` + `tasks.md`，无需回溯外部对话。包含：决策记录（已拍板结论）、现状影响面（逐文件）、布局解析设计、r131 条文改写草案、`specs-flatten` migrate 设计、`--prompt` 机制、测试计划、文档同步清单、未决点。

---

## 1. 决策记录（需求方已拍板）

| # | 决策 | 结论 |
|---|------|------|
| D1 | 布局形态 | 双布局：扁平（新默认）+ 目录（兼容存量），不强制纯扁平零兼容 |
| D2 | 目录化是否保留 | 保留为可选形态；`migrate --kind specs-flatten` 提供一次性平铺，供想统一的项目用 |
| D3 | 命名约定 | CLI 不强制「文件名 == 目录名 == header」；这类约定是各项目自己的约束（容忍 `specs/foo/bar.feature`） |
| D4 | 目录内多 `.feature` | **放宽**：不再 ERROR，降为 Warning（避免卡用户整理期主线，如并排草稿再清理）；主文件 = 同名文件优先 |
| D5 | 同 id 双来源 | 扁平 `specs/foo.feature` 与目录 `specs/foo/`（含主文件）并存 → **ERROR**（保证 resolve 确定性，机器规则，与命名约定无关） |
| D6 | 异名目录（`specs/foo/bar.feature` 且无同名主文件） | `specs-flatten` **跳过并报告**（CLI 不改名） |
| D7 | scope 自引用改写 | migrate 时**自动改写**（纯机械映射；不自动则扁平后 staleness 静默失效） |
| D8 | 迁移预检查 | 「只报告不处理」原则；无任何强制迁移 |
| D9 | 深层嵌套分组 | **先行禁止**（只允许扁平文件与单层目录两种形态），观察反馈后再议 |
| D10 | `--prompt` 作用域 | 先落地 `project migrate`；机制设计为可扩展（其他命令按需加 flag + 模板） |
| D11 | r131 编号 | **保留 r131 编号、改写条文内容**（r136 条文与错误文案多处字面引用 "r131"，换号会断引用链） |
| D12 | `# scope:` 职责 | 保持现状（staleness 扫描范围）；本次只修 migrate 改写与提示，不推广「scope 必须指向源码目录」的新规范 |

## 2. 现状与影响面（今日代码事实，2026-02）

### 2.1 读取侧：三个 choke point，其余全收敛

| 位置 | 现状假设 | 改动 |
|------|---------|------|
| `spec/validation.rs::resolve_spec_file(~225)` | `specs/<id>/` 是目录；恰好一个 `.feature`（多 = ERROR）；`spec.toon` 存在 = legacy 报错指向 toon2features | 改为按 §3 解析规则（双形态 + 优先级 + 冲突 ERROR） |
| `shared/discovery.rs::list_specs(~249)` | `read_dir(specs/)` 只收目录；目录含 `spec.toon` 或 `.feature` 即计入 | 增加扁平文件收集；目录内多文件仍按目录计（id=目录名） |
| `spec/validation.rs(~180)` | `# capability:` 必须 == 目录名（Warning） | 比较对象改为「id」（stem 或目录名），仍 Warning 不阻断 |

以上两处（`resolve_spec_file` / `list_specs`）是唯一入口；`show`/`list`/`validate`/`review`/`req_registry`/`context/index`/`staleness` 全部只消费它们 → **布局改动对上层透明**。

### 2.2 写侧

- `authoring/spec.rs`：`run_skeleton` 写 `specs/<cap>/<cap>.feature` + `mkdir`（~59）；`spec_file_path`（~36）走 `resolve_spec_file`。→ skeleton 默认改扁平（`specs/<cap>.feature`，不建目录）。
- `project/migrate.rs`（toon2features）：`collect_capability_dirs(~506)` 收集目录；产出 `dir/<cap>.feature`。→ 不变；新增 specs-flatten 分支。
- `project/interop.rs`（openspec import）：写 `target/<dir>/…`。→ 不变（import 按 openspec 目录同构，保持目录形态合理）。

### 2.3 栅栏后（无需改动，已验证）

- **BDD harness**：`tests/bdd_steps.rs:1020` `scenarios!("llmanspec/specs", tags="@executable")` —— rstest-bdd 0.6-beta3 用 `WalkDir` **递归**收集 `.feature`（`macro:/scenarios/feature_discovery.rs collect_feature_files`），扁平文件直接被收录。
- **Specs landing**：`change/specs_landing.rs` 只看 `llmanspec/specs` pathspec 前缀下的 `.feature` diff（r130），布局无关。
- **archive/finalize**：ff-merge 整棵 specs 树，路径无关。
- **context index**：按 `list_specs` + 递归 hash（`compute_spec_hash`），无布局假设。
- **staleness scope 匹配**：`spec/staleness.rs::scope_matches(~391)` 前缀匹配（`== scope` 或 `scope/` 前缀）——**扁平化后目录自引用 scope 会失效**，这就是 D7 自动改写的动机。

### 2.4 与布局相关的隐藏耦合：`# scope:` 自引用

- 现状很多 spec 写 `# scope: llmanspec/specs/<cap>`（自己的目录）。扁平后该串匹配不到任何东西（`llmanspec/specs/cli` 不命中 `cli.feature` 也不是 `cli/` 前缀）→ 该 capability 的 staleness 判定静默失效。
- scope 的职责（给实现者的背景）：**staleness 扫描范围声明**。`validate`/`review` 每次对比 git 变更，scope 内任何路径有改动 → 该 capability 标 stale（需求可能脱节）。skeleton 默认 `# scope: src/`；真实语义应指向规范管辖的源码/配置目录，自引用无意义。
- 本次处理：migrate 自动改写自引用项 + 输出提示块（§5.4），不擅自改各 spec 的 scope 语义（D12）。

### 2.5 行号快照（避免实现者重新翻）

- `validation.rs:225-260` resolve_spec_file（含 3 条 r131 措辞错误消息：legacy toon / 无 feature / 多 feature）
- `validation.rs:180-191` `# capability:` 匹配 Warning
- `validation.rs:1382-1393` `discover_features(spec_dir)` —— glob `*.feature` 非递归，保持
- `discovery.rs:249-280` `list_specs`
- `command.rs:347-356` migrate `--kind` value_parser `["toon2features"]` + `:863-869` 分发
- `migrate.rs:99-140` run_at 扫描；`:506-530` `collect_capability_dirs`
- `authoring/spec.rs:57-130` run_skeleton（mkdir + `specs/<cap>/<cap>.feature` + scope 脚手架）
- `change/specs_landing.rs` SPECS_PATHSPEC / r130（不改）
- `staleness.rs:370-405` normalize/scope_matches（migrate 改写复用 normalize 逻辑）

## 3. 布局解析设计（核心）

### 3.1 id 推导

| 物理形态 | cap id | 主文件解析优先级 |
|---------|--------|-----------------|
| `specs/<cap>.feature`（file） | 文件 stem | 自身 |
| `specs/<cap>/`（dir）+ 含同名主文件 | 目录名 | `specs/<cap>/<cap>.feature` |
| `specs/<cap>/` + 无同名、恰 1 个 `.feature` | 目录名 | 该唯一文件（backfill） |
| `specs/<cap>/` + 无同名、多 `.feature` | 目录名（列出所有文件） | 无法唯一解析 → Error（引导人工用 `specs-flatten` 或整理） |

- 目录内多 `.feature`：非主文件仅 Warning（如「非主 feature：harness 资产或草稿？」），validate/list/show 不阻断（D4）。
- legacy `spec.toon`：只以目录形态存在（`specs/<cap>/spec.toon`），检测与错误提示保持现状（指向 toon2features）。

### 3.2 冲突规则（机器确定性，ERROR）

1. `specs/foo.feature` 与 `specs/foo/`（目录内含 `.feature`）并存 → 同 id 双来源 ERROR，报告两个路径。
2. 目录内多 `.feature` 且无同名主文件 → resolve 操作（如 `sdd spec show foo`、`spec add-requirement`）ERROR 提示整理；`list`/`validate` 不因此失败（列出 + Warning）。

### 3.3 实现建议：`SpecLoc` 单源（对齐 `ChangeLoc` 先例）

`shared/discovery.rs` 的 `ChangeLoc { id, path }` 已证明「id + 相对路径」模式（changes 侧同时支持 `some/c0` 与 `c0`）。specs 侧引入对称结构：

```rust
pub(crate) struct SpecLoc {
    pub(crate) id: String,        // stem 或目录名
    pub(crate) path: PathBuf,     // 相对 llmanspec/specs/ 的路径（文件或目录）
}
```

- `list_specs(root) -> Vec<String>` 改为/增补 `list_spec_locs(root) -> Result<Vec<SpecLoc>>`：read_dir 一层，文件（`*.feature` stem=id）与目录（id=目录名，见 §3.1）都收，排序去重，冲突上报。
- `resolve_spec_file` 内部改用 SpecLoc 解析（保持现有公开签名以最小化调用面；或重构调用方拿 `SpecLoc` 作显示路径）。**要求：所有命令的输出路径/错误路径一律走相对 `llmanspec/specs/` 的 SpecLoc.path，禁止再拼 `specs/<id>/` 字符串**（现状 `{feature_path}` 替换与错误消息也有拼装点，见 `validate.rs:1604` 附近与 3 条错误消息）。

## 4. r131 条文改写草案（spec-format.feature `@req:r131`）

> 保留 r131 编号（D11），替换现条文「每个 capability 目录 MUST 仅以单个 .feature …」为：

```
@req:r131 @human
场景: 单轨规格事实源与布局
  - 每个 capability MUST 以恰好一个 .feature 文件作为规格唯一事实源（头注释元数据 + @human 约束场景 + @executable 验收场景）。capability 布局 MUST 二选一：扁平 llmanspec/specs/<cap>.feature（cap id = 文件 stem；新默认，spec skeleton 与 authoring 走此形态）或目录 llmanspec/specs/<cap>/（cap id = 目录名；主文件为同名 .feature，目录内可含其它 .feature 作为 harness 资产/草稿，不计入主文件）。同一 cap id 的扁平与目录两种布局并存 MUST 判为冲突 ERROR。目录内非同名 .feature MUST 仅给 WARNING（不阻断 validate/list/show）。命名约定（文件名 vs 目录名 vs # capability: 头）由项目自约定，CLI 不强制；header 与 cap id 不一致 MUST 仅给 WARNING。运行时 MUST NOT 读取 spec.toon：validate/list/show/context 遇遗留 spec.toon MUST 报 ERROR 并提示执行 llman sdd project migrate --kind toon2features（legacy 仅以目录形态存在）。
```

配套：`spec-format.feature` 增加 `@executable` 场景（见 §7 测试计划种子）；r136 条文里的「按 r131 人工合并」引用保持不变（编号未动）。

## 5. `project migrate --kind specs-flatten` 设计

### 5.1 命令面

- `command.rs` migrate `--kind` value_parser 改为 `["toon2features", "specs-flatten"]`；帮助文案列出两个合法 kind。
- 新参数：`--dry-run`（预览不落盘）、`--yes`（沿用 toon2features 的非交互确认模式）、`--prompt`（见 §6）。
- 与 toon2features 互斥指引：未迁移 legacy `spec.toon` 的目录被 specs-flatten 跳过并报告（先跑 toon2features）。

### 5.2 处理范围（只动这一种）

`specs/<cap>/` 目录同时满足：

- 内含**恰好一个** `.feature`；
- 该 `.feature` 文件名 == 目录名（`<cap>.feature`）；
- 目录内无其它文件（不含 `spec.toon`、非 `.feature` 文件、隐藏文件）。

处理动作（按序，单目录原子化）：

1. `git mv specs/<cap>/<cap>.feature specs/<cap>.feature`（git 保留历史；非 git 环境 fallback `fs::rename`）。
2. 目录内 `# scope:` 自引用改写（§5.4）——在 mv 前完成内容改写，然后整体 mv。
3. 删除空目录 `specs/<cap>/`（`fs::remove_dir`，若失败仅警告不失败整体）。

### 5.3 预检查清单（全部只报告、不处理、不失败整体）

| # | 条件 | 报告类别 | 处理 |
|---|------|---------|------|
| 1 | `specs/<cap>.feature` 已存在（与另一扁平文件或本次目标重名） | `conflict` | 跳过（保留两文件） |
| 2 | 目录含 `spec.toon` | `legacy` | 跳过；提示先 `toon2features` |
| 3 | 目录含多个 `.feature` | `multi` | 跳过（用户整理期状态，D4） |
| 4 | 目录含非 `.feature` 文件（notes/截图/hidden） | `aux` | 跳过（保留目录） |
| 5 | 目录含唯一 `.feature` 但文件名 ≠ 目录名 | `misnamed` | 跳过（CLI 不改名，D6） |
| 6 | 已经是扁平或不存在 | `already-flat` / 无 | no-op，幂等 |

报告输出（照 toon2features 风格）：`flattened` / `conflict` / `legacy` / `multi` / `aux` / `misnamed` 计数 + 每项相对路径；退出码：全部扁平或 no-op → 0；存在 skipped 项 → 0（报告为主，同 toon2features 的 skipped 语义；如需 CI 感知可后续加 `--strict`，本次不做）。

### 5.4 scope 自引用改写（D7）

- 匹配：文件的 `# scope:` 值列表（按逗号分隔、normalize 后——trim、去 `./`/`/` 前缀、去尾 `/`，复用 `staleness.rs::normalize_scope_list` 语义）中，有项精确等于 `llmanspec/specs/<cap>`。
- 改写：该项替换为 `llmanspec/specs/<cap>.feature`（保住「包含自身」原意；只改这一项，其它 scope 项不动）。
- 报告：打印被改写的目录路径 + 新旧值（计入 `scope_rewritten` 计数）。

### 5.5 输出示例（给实现者的形态参考）

```
$ llman sdd project migrate --kind specs-flatten --yes
flattened 12  (llmanspec/specs/*.feature …)
scope_rewritten 12  (llmanspec/specs/cli: # scope: llmanspec/specs/cli → llmanspec/specs/cli.feature)
skipped 3
  - conflict  llmanspec/specs/foo/       (target llmanspec/specs/foo.feature exists)
  - legacy    llmanspec/specs/legacy/    (spec.toon present; run --kind toon2features first)
  - misnamed  llmanspec/specs/bar/       (bar/other.feature; CLI won't rename)

— scope 检查 —
`# scope:` 声明每个 capability 的 staleness 扫描范围（validate/review 对比 git 变更判定 stale）。
建议：把 scope 指向该规范管辖的真实源码/配置目录（如 src/sdd/…），而非规范文件自身。
改写项已自动同步为 specs/<cap>.feature；改动后建议运行 `llman sdd validate --specs` 确认无 stale WARNING。
```

## 6. 协作提示（`--prompt` + 执行输出注入）

### 6.1 `--prompt` flag（先落地 `project migrate`，D10）

- 语义：`llman sdd project migrate --prompt`（可与 `--kind` 组合）**只打印内置协作说明并返回 0，不执行任何迁移**。
- 模板托管：`crates/llman-sdd/templates/sdd/{locale}/units/migrate-prompt.md`（zh-Hans/en 双语，与 skill 模板同构、同 `include_str!` 嵌入、同 check-sdd-templates 门禁）。已有 `units/` 下 `validation-hints.md`、`feature-contract.md` 先例。
- 内容结构（agent 与人类都可读）：

```
# llman sdd project migrate — 协作说明
## 命令意图
  toon2features: 遗留 spec.toon → 单轨 .feature（一次性）
  specs-flatten:  单文件单目录 specs/<cap>/<cap>.feature → 扁平 specs/<cap>.feature（保留历史）
## Agent 该做什么
  - 先确认是否真需要迁移（无 legacy/dir 形态则 no-op）
  - 先跑 --dry-run 看预检查报告；conflict/misnamed 项人工处理，勿用 --force 硬来
  - 迁移后跑 validate --specs 与 BDD（cargo test --features bdd）
## 人类该做什么
  - 检查 scope_rewritten 报告与 git diff（mv 保留历史）
  - 顺手把 # scope: 指向真实源码目录（见下方说明）
## 陷阱
  - 异名目录/多文件/附属文件目录不会自动扁平（只报告）
  - 重名冲突必须人工解决（两文件都保留）
  - scope 自引用会被自动改写为 specs/<cap>.feature
## 下一步
  - llman sdd validate --specs --strict --no-interactive
  - cargo test --features bdd
```

### 6.2 执行输出注入

- `--prompt` 之外，migrate **实际执行**时（toon2features 与 specs-flatten 皆有受益，本次至少 specs-flatten）在报告末尾追加「— scope 检查 —」块（见 §5.5），解释 scope 职责 + 建议 + 下一步。toon2features 是否也注入由实现者酌情（其不触碰 scope，可仅一行提示）。

### 6.3 可扩展性

机制 = 子命令 flag + `units/*-prompt.md` 模板 + 分发点打印即返回。后续任何命令加 `--prompt` 只需：clap flag + 模板 + 两行分发，无需新框架。

## 7. 测试计划

### 7.1 spec-format.feature `@executable` 场景（验收种子，落到实现者绑定分支上）

挂 `@req:r131` 的：
1. `specs-flat-file-is-a-capability`：扁平 `specs/sample.feature` 被 `list`/`show`/`validate` 识别（id=stem）。
2. `flat-and-dir-collision-errors`：`specs/foo.feature` 与 `specs/foo/foo.feature` 并存 → `list`/`validate` 非零退出且报两个路径。
3. `multi-feature-dir-warns-not-fails`：目录内主文件 + 第二 `.feature` → validate 零退出、含 Warning；show <cap> 正常。
4. `dir-without-main-resolves-single`：`specs/foo/bar.feature` 唯一文件 → id=foo backfill 可 show/validate 其内容。

挂 `@req:flutter-migrate`（新 rule id，实现者分配，如 r138）的：
5. `flatten-converts-single-dir`：`specs/<cap>/<cap>.feature` 目录 → migrate 后 `specs/<cap>.feature` 存在、目录不存在、git mv 保留。
6. `flatten-reports-every-skip-class`：conflict/legacy/multi/aux/misnamed 各一 → 退出 0、计数组件齐全、文件未被移动。
7. `flatten-rewrites-self-scope`：`# scope: llmanspec/specs/<cap>` → 内容包含 `<cap>.feature`。
8. `flatten-dry-run-noop`：`--dry-run` 后目录与文件均未变。
9. `migrate-prompt-prints-and-skips`：`--prompt` → stdout 含「协作说明」要点、退出 0、无文件变动。

### 7.2 Rust 测试

- `sdd_bdd_compat.rs`（实现细节层，AGENTS.md 规定的同步义务）：skeleton 默认扁平（产出 `specs/<cap>.feature` 无目录）；migrate kind 解析（`["toon2features","specs-flatten"]`，`spec-md2toon` 仍拒绝）；`resolve_spec_file` 单元测试补齐优先级矩阵（§3.1 表）。
- 现有 fixture（`bdd_steps.rs` 的 sample/sample3/legacy）保持目录形态场景，验证兼容未回归。

## 8. 文档同步清单（实现者逐项勾）

- [ ] `llmanspec/specs/spec-format/spec-format.feature`：r131 改写 + 新增 migrate rule（r138 起序）+ 上面 9 个 @executable 场景
- [ ] r136 条文内「按 r131 人工合并」引用核对（编号保留则无需改）
- [ ] 根 `AGENTS.md`「单轨格式」段（r131-r136 描述行 + `specs/<capability>/<capability>.feature` 各处字样）
- [ ] `crates/llman-sdd/templates/sdd/{zh-Hans,en}/units/validation-hints.md`、`units/feature-contract.md`
- [ ] `crates/llman-sdd/templates/sdd/{zh-Hans,en}/skills/*`（propose/apply/verify/continue/ff 等含 `specs/<capability>/<capability>.feature` 字样处；建议统一改为 `specs/<capability>.feature（或目录形态）`）
- [ ] `crates/llman-sdd/templates/sdd/{zh-Hans,en}/units/migrate-prompt.md`（新增）
- [ ] `command.rs` skeleton 帮助（`llmanspec/specs/<cap>/<cap>.feature` → 扁平）+ migrate 帮助（两个 kind）
- [ ] `authoring/spec.rs` skeleton 输出消息与 `spec_dir.exists()` 冲突检查（扁平：`specs/<cap>.feature` 存在即拒，除非 --force）
- [ ] 运行 `just check-sdd-templates`（本仓库）与 `just readme`（若 CLI 帮助文案有生成位）

## 9. 风险与未决点

1. **header 与 id 不一致的 Warning 文案**：现文案写死 "must match spec directory name"，双形态后需改为 "…spec id（目录名或文件 stem）"——影响 `t!` 键与既有 BDD 断言，实现者注意 grep 旧文案。
2. **specs-flatten 与未 commit 工作树**：`git mv` 前应检测目标目录是否被 git 跟踪（untracked 文件 mv 后无历史，提示人工确认）；fail-open（报告即可）。
3. **退出码语义**：skipped 类不非零退出（同 toon2features 的 skipped）。若需求方后续要 CI 硬门禁，加 `--strict` 即可，本次不做（D8）。
4. **目录形态上限**：本次与未来都**不**支持深层嵌套（`specs/group/cap/cap.feature`，D9）；若未来需要，解析层已收口（SpecLoc），加 depth 是纯增量。
5. **仓库本体是否立即跑 specs-flatten**：建议作为独立后续 change（本 change 只交付机制 + 本仓库同步文档），避免把迁移噪音混进行为变更；实现者可在实现完成后另开 quick change 跑一次全库扁平并核对 scope 改写。

## 10. 参考

- 现状数据：本仓库 20+ capability 目录全部「每目录恰 1 同名文件」（2026-02 快照核对无 EXTRA/MISMATCH）。
- `ChangeLoc` 先例：`crates/llman-sdd/src/sdd/shared/discovery.rs`（changes 嵌套 + 扁平并存）。
- rstest-bdd 0.6.0-beta3 `scenarios!`：`WalkDir` 递归收集（宏源码 `feature_discovery.rs::collect_feature_files`）——扁平被天然支持。
