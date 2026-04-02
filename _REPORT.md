# SDD 多风格 spec/delta 风格处理分析报告（基于真实 OpenSpec 工件）

日期：2026-04-02
作者：Codex（自动分析生成）

---

## 0. 报告目标

你提到 “最近对 SDD 进行了大更新”，并希望看到：

1) 使用 **真实** `openspec/specs/**` 与 `openspec/changes/**` 的例子（内容要“比较长”）
2) 基于这些工件，**逻辑推理**出 SDD 的“风格处理”（`ison / toon / yaml`）应如何工作
3) 输出一个可阅读的分析报告，并给出 **token 量的推断**

本报告以仓库内的真实工件为输入，给出：

- 从 OpenSpec spec/changes **抽象出的语义 IR**（中间表示）
- 三种风格（`ison/toon/yaml`）的 **envelope 约束**、**解析/写回策略**、**转换(convert)门禁**
- 一组 **等语义**的 main spec 与 delta spec（变化 spec）在三种风格下的示例
- 对上述示例的 **token 粗估**（含方法与误差说明）

---

## 1. 选取的真实输入工件（Openspec）

### 1.1 Change（OpenSpec change，较长）

选用归档 change：`openspec/changes/archive/2026-04-01-support-multi-style-sdd-specs/`

- `openspec/changes/archive/2026-04-01-support-multi-style-sdd-specs/proposal.md`
- `openspec/changes/archive/2026-04-01-support-multi-style-sdd-specs/design.md`
- `openspec/changes/archive/2026-04-01-support-multi-style-sdd-specs/tasks.md`

这组文档非常典型：它同时给出 **为什么做**、**设计约束**、**实现拆分与测试点**，足以推导出多风格处理的“必须正确工作”的逻辑。

### 1.2 Specs（OpenSpec capability specs）

为了把“风格处理”讲清楚，报告使用了两份 spec：

1) **多风格行为约束（核心）**
   `openspec/specs/sdd-multi-style-formats/spec.md`

2) **三风格的 canonical 结构契约（定义主 spec/delta spec 的结构）**
   `openspec/specs/sdd-ison-authoring/spec.md`

> 说明：你原话希望“用一个真实 spec”，但在 SDD 多风格场景里，`sdd-multi-style-formats` 负责“风格门禁/转换/语义一致性”，而 `sdd-ison-authoring` 负责“canonical payload 的结构契约”。两者一起才能完整推导出“解析/验证/写回/转换”的闭环；报告会明确区分它们的作用。

---

## 2. 从 OpenSpec 推导出的 “共享语义 IR”（中间表示）

### 2.1 为什么必须有 IR

`support-multi-style-sdd-specs` 的设计核心（见 `design.md`）是：

- **风格差异只停留在 envelope（fence + 语法）层**
- 命令（`show/list/validate/archive`、authoring helpers、convert）应消费 **同一个语义模型**，避免复制三套业务逻辑

这意味着系统必然需要一个 style-agnostic 的 IR（中间表示），并且三种风格都必须可逆地映射到它。

### 2.2 IR 结构（从 `sdd-multi-style-formats` 的需求直接抽取）

#### MainSpecIR（主 spec）

- `kind`（必须为 `llman.sdd.spec`）
- `name`（strict 模式下必须等于 `<capability>`）
- `purpose`
- `requirements[]`：每项含 `req_id/title/statement`
- `scenarios[]`：每项含 `req_id/id/given/when/then`

#### DeltaSpecIR（delta spec / change spec）

- `kind`（必须为 `llman.sdd.delta`）
- `ops[]`：每项含 `op/req_id/title/statement/from/to/name`
- `op_scenarios[]`：每项含 `req_id/id/given/when/then`

> 这两份 IR 的字段集合，与 `openspec/specs/sdd-multi-style-formats/spec.md` “三种风格必须共享同一语义模型” 部分一致。

---

## 3. 风格处理的关键逻辑（从 change + specs 逻辑推断）

这里把“风格处理”拆成 4 个层次：**配置门禁** → **envelope 识别** → **语义解析/校验** → **写回/转换**。

### 3.1 配置门禁（`spec_style` 必须显式声明）

从 `openspec/specs/sdd-multi-style-formats/spec.md` 与 `...support-multi-style.../design.md` 可以推出以下硬门禁：

- `llmanspec/config.yaml` **必须**显式声明：`spec_style: ison|toon|yaml`
- 对 “读取或改写 spec/delta payload” 的命令：
  - 缺失 / 为空 / 非法 `spec_style` → **直接失败**（不允许“默认 ison 继续”）
- 已声明 `spec_style` 的项目：
  - 遇到不同风格 fence / payload → **直接失败**，并给出 expected vs found

这保证了：项目风格是单一的、可审计的，避免“同仓库混用多语法”导致的非确定性。

### 3.2 Envelope 识别（fence 的严格匹配）

从 `sdd-multi-style-formats` 得到 “风格→fence” 对应关系：

- `ison` → ` ```ison `
- `toon` → ` ```toon `
- `yaml` → ` ```yaml `

并推出 envelope 处理策略：

1) 从 `spec.md`（Markdown）中提取 fenced blocks
2) 严格检查“出现了哪些 fence”：
   - 若项目风格为 `yaml`，但文件只含 ` ```ison ` → fail（明确指出 mismatch）
3) 若风格为 `ison`：
   - **允许多个** ` ```ison ` block（`sdd-ison-authoring` 明确要求支持多 block 合并）
4) 若风格为 `toon/yaml`：
   - **必须是一个** canonical 文档（单 fence），避免引入风格专属的 block merge 规则

### 3.3 语义解析 + 校验（风格无关的语义一致性）

由 `sdd-ison-authoring` 可以推出两层校验：

1) **Envelope 校验（风格相关）**
   - fence 类型正确
   - 对 `ison`：canonical block 名称、列名、重复 block 等错误

2) **IR 校验（风格无关）**
   - `kind` 正确（spec/delta）
   - strict：`name == capability`
   - 每个 requirement ≥ 1 个 scenario
   - `(req_id, id)` 唯一
   - delta op 的规则（`add/modify/remove/rename` 对字段的约束）

关键点：只有 envelope 部分依赖风格，IR 校验必须完全一致，这样 `show/list/validate/archive` 才能跨风格等价。

### 3.4 写回策略（尤其是 YAML 的“lossless 优先”）

从 `support-multi-style.../proposal.md` 与 `design.md` 的 YAML 段落可推出：

- `ison`：默认 token-friendly dump（不做列对齐），可选 pretty 对齐（仅限 ison）
- `toon`：稳定、严格的 canonical 编码（固定顺序、固定结构）
- `yaml`：
  - 优先尝试 **lossless overlay**（保留注释/格式/键顺序尽量不动）
  - overlay 应以语义 ID（如 `req_id`、`(req_id,id)`）为锚点生成 patch 计划
  - overlay 失败 → 回退为“仅重写 fenced YAML payload 的确定性重写”（Markdown 外围保留，但 payload 内注释可能丢失）

这解释了为什么 YAML 后端比 ISON/TOON 更复杂：它承担了“人类可维护 YAML spec”的编辑体验目标。

---

## 4. Worked Example（等语义：三风格 main spec + 三风格 delta spec）

为了对齐你想看的 “不同形态”，下面用 `sdd-multi-style-formats` 这份真实 capability spec 的语义，构造一份 **等语义** main spec，并用 change `support-multi-style-sdd-specs` 构造一份 delta spec 示例（表示在 `sdd-workflow` 里新增了两条要求：风格门禁 + convert）。

> 注意：`req_id` / `scenario id` 在 OpenSpec Markdown 里并不是强制字段，本报告为了演示 SDD canonical payload，给这些 requirement/scenario 人工分配了稳定 ID（这也是 SDD 结构化写法的一个关键收益：可引用、可 patch、可验证）。

### 4.1 共同的 main spec wrapper（frontmatter + 标题）

在 SDD 工作流里，main spec 通常带 frontmatter（见 `openspec/specs/sdd-workflow/spec.md` 的 “Spec 校验元数据” 要求）。三种风格**都共用**同一套 wrapper；只有 fenced payload 不同。

```md
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - just test
llman_spec_evidence:
  - tests/sdd_integration_tests.rs
---

# sdd-multi-style-formats
```

---

### 4.2 Main spec：`spec_style: ison`（canonical table/object ISON）

```ison
object.spec
kind name purpose
"llman.sdd.spec" sdd-multi-style-formats "项目级显式选择 ison/toon/yaml，并保持严格风格门禁。"

table.requirements
req_id title statement
r1 "显式 spec_style" "`llmanspec/config.yaml` MUST 显式声明 `spec_style` 且值 MUST 为 `ison|toon|yaml`；缺失/非法 MUST 失败并提示修复。"
r2 "风格严格匹配" "主 spec 与 delta spec 的 canonical payload fence MUST 与项目 `spec_style` 严格一致；不得自动探测/回退。"
r3 "统一语义模型" "无论 `ison|toon|yaml`，解析后 MUST 归一化到同一语义 IR，再驱动 show/list/validate/archive。"
r4 "显式 convert" "系统 MUST 提供显式风格转换（项目/单文件），并在写入前后重解析验证语义等价；失败 MUST 不更新配置。"
r5 "标记 experimental" "`toon` 与 `yaml` MUST 在帮助/错误/模板中标记为 experimental。"

table.scenarios
req_id id given when then
r1 init_has_style "" "运行 `llman sdd init`" "生成的 `llmanspec/config.yaml` 包含 `spec_style: ison`"
r1 missing_style_blocks "" "项目缺失 `spec_style` 且运行 `llman sdd show sample --type spec`" "命令失败并提示先设置 `spec_style`"
r2 toon_rejects_ison "" "项目声明 `spec_style: toon` 但 spec 文件只有 ` ```ison `" "validate 失败并指出 expected=toon found=ison"
r2 mixed_styles "" "主 spec 使用 `yaml` 且某个 delta spec 使用 `toon`" "validate --changes 失败并指出不允许混用"
r3 show_semantic_equal "" "同一语义分别以 ison/yaml/toon 编写" "show --json 的语义字段一致"
r3 archive_semantic_equal "" "同一语义分别以 ison/yaml 编写并归档" "archive merge 后语义结果一致"
r4 convert_project_success "" "执行项目范围 convert 并全部重解析通过" "文件被重写且最后更新 `llmanspec/config.yaml`"
r4 convert_project_fail "" "转换后重解析某文件失败" "命令失败且 `llmanspec/config.yaml` 保持旧值"
r5 help_marks_experimental "" "查看帮助/模板示例" "明确标注 toon/yaml 为 experimental"
```

#### ISON 处理要点（从 `sdd-ison-authoring` 推导）

- canonical block 名称必须固定：`object.spec` / `table.requirements` / `table.scenarios`
- 允许把三个 block 分散在多个 ` ```ison ` fence 中（按 block name 合并），但 **同名 block 重复即错误**
- 默认 dump 要 token-friendly：不做列对齐填充（否则 padding 会膨胀 token）

---

### 4.3 Main spec：`spec_style: yaml`（canonical YAML doc，experimental）

```yaml
kind: llman.sdd.spec
name: sdd-multi-style-formats
purpose: '项目级显式选择 ison/toon/yaml，并保持严格风格门禁。'
requirements:
- req_id: r1
  title: 显式 spec_style
  statement: llmanspec/config.yaml MUST 显式声明 spec_style 且值 MUST 为 ison|toon|yaml；缺失/非法 MUST 失败并提示修复。
- req_id: r2
  title: 风格严格匹配
  statement: 主 spec 与 delta spec 的 canonical payload fence MUST 与项目 spec_style 严格一致；不得自动探测/回退。
- req_id: r3
  title: 统一语义模型
  statement: 无论 ison|toon|yaml，解析后 MUST 归一化到同一语义 IR，再驱动 show/list/validate/archive。
- req_id: r4
  title: 显式 convert
  statement: 系统 MUST 提供显式风格转换（项目/单文件），并在写入前后重解析验证语义等价；失败 MUST 不更新配置。
- req_id: r5
  title: 标记 experimental
  statement: toon 与 yaml MUST 在帮助/错误/模板中标记为 experimental。
scenarios:
- req_id: r1
  id: init_has_style
  given: ''
  when: 运行 llman sdd init
  then: 生成的 llmanspec/config.yaml 包含 spec_style: ison
- req_id: r1
  id: missing_style_blocks
  given: ''
  when: 项目缺失 spec_style 且运行 llman sdd show sample --type spec
  then: 命令失败并提示先设置 spec_style
- req_id: r2
  id: toon_rejects_ison
  given: ''
  when: 项目声明 spec_style: toon 但 spec 文件只有 ```ison
  then: validate 失败并指出 expected=toon found=ison
- req_id: r2
  id: mixed_styles
  given: ''
  when: 主 spec 使用 yaml 且某个 delta spec 使用 toon
  then: validate --changes 失败并指出不允许混用
- req_id: r3
  id: show_semantic_equal
  given: ''
  when: 同一语义分别以 ison/yaml/toon 编写
  then: show --json 的语义字段一致
- req_id: r3
  id: archive_semantic_equal
  given: ''
  when: 同一语义分别以 ison/yaml 编写并归档
  then: archive merge 后语义结果一致
- req_id: r4
  id: convert_project_success
  given: ''
  when: 执行项目范围 convert 并全部重解析通过
  then: 文件被重写且最后更新 llmanspec/config.yaml
- req_id: r4
  id: convert_project_fail
  given: ''
  when: 转换后重解析某文件失败
  then: 命令失败且 llmanspec/config.yaml 保持旧值
- req_id: r5
  id: help_marks_experimental
  given: ''
  when: 查看帮助/模板示例
  then: 明确标注 toon/yaml 为 experimental
```

#### YAML 处理要点（从 change 设计推导）

- 解析：把 fenced YAML 解析到 IR（字段必须齐全、顺序不影响语义）
- 写回：优先 lossless overlay（语义锚点：`req_id`、`(req_id,id)`）；失败再 fallback 到 deterministic rewrite

---

### 4.4 Main spec：`spec_style: toon`（canonical TOON doc，experimental）

```toon
kind: llman.sdd.spec
name: sdd-multi-style-formats
purpose: "项目级显式选择 ison/toon/yaml，并保持严格风格门禁。"
requirements[5]{req_id,title,statement}:
  r1,"显式 spec_style","llmanspec/config.yaml MUST 显式声明 spec_style 且值 MUST 为 ison|toon|yaml；缺失/非法 MUST 失败并提示修复。"
  r2,"风格严格匹配","主 spec 与 delta spec 的 canonical payload fence MUST 与项目 spec_style 严格一致；不得自动探测/回退。"
  r3,"统一语义模型","无论 ison|toon|yaml，解析后 MUST 归一化到同一语义 IR，再驱动 show/list/validate/archive。"
  r4,"显式 convert","系统 MUST 提供显式风格转换（项目/单文件），并在写入前后重解析验证语义等价；失败 MUST 不更新配置。"
  r5,"标记 experimental","toon 与 yaml MUST 在帮助/错误/模板中标记为 experimental。"
scenarios[9]{req_id,id,given,when,then}:
  r1,init_has_style,"","运行 llman sdd init","生成的 llmanspec/config.yaml 包含 spec_style: ison"
  r1,missing_style_blocks,"","项目缺失 spec_style 且运行 llman sdd show sample --type spec","命令失败并提示先设置 spec_style"
  r2,toon_rejects_ison,"","项目声明 spec_style: toon 但 spec 文件只有 ```ison","validate 失败并指出 expected=toon found=ison"
  r2,mixed_styles,"","主 spec 使用 yaml 且某个 delta spec 使用 toon","validate --changes 失败并指出不允许混用"
  r3,show_semantic_equal,"","同一语义分别以 ison/yaml/toon 编写","show --json 的语义字段一致"
  r3,archive_semantic_equal,"","同一语义分别以 ison/yaml 编写并归档","archive merge 后语义结果一致"
  r4,convert_project_success,"","执行项目范围 convert 并全部重解析通过","文件被重写且最后更新 llmanspec/config.yaml"
  r4,convert_project_fail,"","转换后重解析某文件失败","命令失败且 llmanspec/config.yaml 保持旧值"
  r5,help_marks_experimental,"","查看帮助/模板示例","明确标注 toon/yaml 为 experimental"
```

#### TOON 处理要点（从 change 设计推导）

- 单 fence 单文档，避免 TOON 引入“多 block 合并规则”
- 稳定序列化：字段顺序、数组顺序确定（否则 diff 噪音会很大）

---

### 4.5 Delta spec（change 中的 `spec.md`）：三风格示例

下面给出一个 delta spec 的“变化语义”示例：假设 change `support-multi-style-sdd-specs` 要在 `sdd-workflow` 里新增两条 requirement：

- `style_gate`：要求 spec 相关命令必须显式 `spec_style`（缺失/非法直接失败）
- `convert`：要求提供 `llman sdd convert` 并验证语义等价

> 这与 `openspec/changes/archive/2026-04-01-support-multi-style-sdd-specs/design.md` 的核心约束一致：strict gating + explicit convert。

#### 4.5.1 `spec_style: ison` delta payload

```ison
object.delta
kind
"llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement style_gate "Spec style gating" "Spec read/write commands MUST require explicit `spec_style`; missing/invalid MUST fail with a concrete hint." ~ ~ ~
add_requirement convert "Explicit convert" "System MUST provide `llman sdd convert` for audited migration between `ison|toon|yaml` and MUST verify semantic equivalence." ~ ~ ~

table.op_scenarios
req_id id given when then
style_gate missing_config "" "user runs `llman sdd show` without configured `spec_style`" "command fails and explains how to set `spec_style`"
convert project_success "" "user runs `llman sdd convert --to yaml --project` and all files reparse" "converted files are written and config is updated last"
```

#### 4.5.2 `spec_style: yaml` delta payload

```yaml
kind: llman.sdd.delta
ops:
- op: add_requirement
  req_id: style_gate
  title: Spec style gating
  statement: Spec read/write commands MUST require explicit spec_style; missing/invalid MUST fail with a concrete hint.
  from: null
  to: null
  name: null
- op: add_requirement
  req_id: convert
  title: Explicit convert
  statement: System MUST provide llman sdd convert for audited migration between ison|toon|yaml and MUST verify semantic equivalence.
  from: null
  to: null
  name: null
op_scenarios:
- req_id: style_gate
  id: missing_config
  given: ''
  when: user runs llman sdd show without configured spec_style
  then: command fails and explains how to set spec_style
- req_id: convert
  id: project_success
  given: ''
  when: user runs llman sdd convert --to yaml --project and all files reparse
  then: converted files are written and config is updated last
```

#### 4.5.3 `spec_style: toon` delta payload

```toon
kind: llman.sdd.delta
ops[2]{op,req_id,title,statement,from,to,name}:
  add_requirement,style_gate,"Spec style gating","Spec read/write commands MUST require explicit spec_style; missing/invalid MUST fail with a concrete hint.",null,null,null
  add_requirement,convert,"Explicit convert","System MUST provide llman sdd convert for audited migration between ison|toon|yaml and MUST verify semantic equivalence.",null,null,null
op_scenarios[2]{req_id,id,given,when,then}:
  style_gate,missing_config,"","user runs llman sdd show without configured spec_style","command fails and explains how to set spec_style"
  convert,project_success,"","user runs llman sdd convert --to yaml --project and all files reparse","converted files are written and config is updated last"
```

---

## 5. Token 量推断（基于 worked example 的粗估）

### 5.1 方法说明（为什么只能粗估）

不同模型使用不同 tokenizer；在不引入模型专用 tokenizer 的前提下，本报告采用启发式估算：

- ASCII 字符按 ~4 chars/token
- CJK 与其他非 ASCII 字符按 ~1 char/token
- **不计空白字符**（因此对 `pretty` 对齐类输出会有低估）

所以结果应按 **±20%** 的误差带来理解：用来比较风格之间的相对大小是可靠的，用来做严格预算不可靠。

### 5.2 估算结果（仅 worked example 的 canonical payload / wrapper）

对 4.2～4.5 中的示例文本做估算，结果如下（单位：tokens，越小越省）：

| 文档 | ison | yaml | toon |
| --- | ---:| ---:| ---:|
| main spec（仅 payload） | ~548 | ~600 | ~549 |
| main spec（含 frontmatter+标题） | ~584 | ~636 | ~584 |
| delta spec（仅 payload） | ~169 | ~180 | ~174 |

**结论（对这个示例）：**

- `yaml` 因为重复 key（`req_id/title/...`）带来额外结构开销，token 更大
- `ison` 与 `toon` 在“表格/行式结构”上更接近，token 更省且更稳定
- 当 requirements/scenarios 数量继续增长时，`yaml` 的结构开销通常增长更快

---

## 6. 风格选择建议（基于 change 的约束 + token 推断）

结合 `support-multi-style-sdd-specs` 的目标（strict gating、统一 IR、确定性写回、显式 convert）和上面的 token 粗估：

- 默认推荐：`ison`
  - 优点：最贴合 SDD 的 row-level 编辑、最 token-friendly、允许多 block 分段组织（但仍然保持 canonical 结构）
  - 风险：需要遵守 canonical block/列名；旧的 JSON-in-ison 会被拒绝（这是刻意的“强收敛”）

- 想要更直读：`yaml`（experimental）
  - 优点：对人类更直观（尤其是新手）
  - 代价：token 更大；写回要做 overlay 以保留注释/格式，复杂度更高

- 想要更紧凑但又结构化：`toon`（experimental）
  - 优点：紧凑、结构化、比 YAML 更少结构冗余
  - 代价：语法更小众；需要强约束 canonical emitter 才能避免 diff 噪音

---

## 7. 你接下来想看什么“形态”？

本报告已经给了：

- 真实 openspec change + specs 的约束提炼
- 一套等语义三风格 main/delta spec 示例
- token 粗估

如果你想进一步对比“更长、更接近真实仓库的 spec”，我可以把 `openspec/specs/sdd-workflow/spec.md` 中某个更大的子集（比如 `init/update/validate/archive` 一整段）抽取成 IR，并生成三风格 canonical payload，再给出 token 对比表（会明显更长、更有冲击力）。
