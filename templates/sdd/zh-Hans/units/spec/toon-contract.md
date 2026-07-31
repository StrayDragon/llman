## Canonical TOON Spec Contract

SDD 主 spec 以**独立的 `.toon` 文件**承载——每个文件一份 TOON 文档，没有 Markdown 外壳，也没有 fenced code block。所有结构化信息（包括原先位于 YAML frontmatter 的校验元数据）都在 TOON 文档内部。

### Main spec（`llmanspec/specs/<feature-id>/spec.toon`）

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[2]: src/,tests/
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

- `kind` 必须为 `llman.sdd.spec`。
- `name` 应与 spec 目录名一致。
- `valid_scope` 是校验作用域（驱动 staleness 检查）。必须存在且非空，为扁平单列表格化数组（例如 `valid_scope[2]: src/,tests/`）。（`valid_commands` 与 `evidence` 已移除——仅有 `valid_scope` 被实际消费。）

### Partitioned SSOT（配置了 `bdd:` 时）

当 `config.yaml` 定义了 `bdd` 块时，`bdd:` 段是**仅 runner 开关**（`validate --check` 执行 `bdd.run_command`）；**不会**分叉变更生命周期。可执行场景采用 **Partitioned SSOT**：

| 层 | 权威 | 内容 |
|---|---|---|
| 约束 | `spec.toon` | `requirements` + **不可执行** scenarios（`feature: false`） |
| Harness | `*.feature` | 可执行 GWT 唯一正文；场景带 `@req:<req_id>` |

```toon
kind: llman.sdd.spec
name: sample
purpose: "约束在 toon；可执行例子在 .feature。"
valid_scope[1]: llmanspec/specs/sample
requirements[1]{req_id,title,statement}:
  r1,新增需求,系统 MUST 完成新功能。
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,internal-only,"管理器扫描","内部检查","通过",false
```

```gherkin
# sample.feature
功能: sample
  @req:r1
  场景: happy
    假如 llman 二进制已构建
    当 运行 llman sample --flag
    那么 退出码为 0
```

- **Git-native 生命周期**：先用 `llman sdd change start <id>`（推荐；须在默认分支干净树）或 `change attach` 完成 Branch binding 进入 Full；**再**在绑定的非默认分支上编辑 live `.feature` 与 `spec.toon` 并 commit（Specs landing）。优先 `change finalize` 单 commit 收尾（或 fallback：归档前 `checkpoint` 再 `change archive`）。archive/finalize 自动 ff-merge feature 分支到默认分支，再将 change 文档改名到 `changes/archive/`（脏改名留一次 `git commit`）。`diff` 只读审查/导出。**禁止**在 `changes/<id>/specs/` 下编写或创建 `*.feature.delta.toon`。没有 `change delta`、solidify 或 `llman-sdd-sync`。
- 下游升级：人工清理遗留 `change/specs/` 或 `*.feature.delta.toon`（`partitioned` migrate 已移除）。
- `bdd:` 已启用且 `requirements` 为空、又无 `.feature` 是 ERROR。

### 表格化行的引号规则

在表格化数组行中（值以逗号分隔），如果值包含**空格**、**逗号**、**冒号**、**方括号**（`[`, `]`, `{`, `}`）或首尾有空白字符，**必须使用双引号包裹**：

```
# 错误：未加引号的空格值会被拆成多个值
r1,happy,"",a trigger happens,the outcome is observed

# 正确：多词值加引号
r1,happy,"","a trigger happens","the outcome is observed"
```

- 空字符串：`""`
- 未设置的可选字段：`null`
- 不确定时，优先使用引号。

### 备注
- 每个 spec 一个 `.toon` 文件；没有 Markdown，没有 ```` ```toon ```` fence。
- `null` 表示该字段缺失（可选字段未设置）。
- 从旧版 `.md`+fence 迁移请使用 `llman sdd migrate`。
