## Canonical TOON Spec Contract

SDD 主 spec 与 delta spec 都以**独立的 `.toon` 文件**承载——每个文件一份 TOON 文档，没有 Markdown 外壳，也没有 fenced code block。所有结构化信息（包括原先位于 YAML frontmatter 的校验元数据）都在 TOON 文档内部。

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

### Main spec BDD-on（Partitioned SSOT）

当 `config.yaml` 定义了 `bdd` 块时采用 **Partitioned SSOT**：

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

- **BDD-on（Git-native）**：在非默认 feature 分支上编辑 live `.feature` 与 `spec.toon`；用 `llman sdd change attach` 绑定；归档前 `checkpoint`；`diff` 只读审查/导出。合并前 `change archive` 仅移动 change 文档——Git/PR merge 提升 specs。**禁止**编写 `*.feature.delta.toon`（遗留活跃 feature_delta 是迁移阻断项）。没有 solidify 命令。
- **BDD-off**：使用下方 Delta 节的 change 内 TOON delta（`ops` / `op_scenarios`）与 archive 合并——不要求 attach/checkpoint/harness。
- 下游升级：`llman sdd project migrate --kind partitioned`。
- BDD 已启用且 `requirements` 为空、又无 `.feature` 是 ERROR。

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`）——BDD-off / 经典

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

- `kind` 必须为 `llman.sdd.delta`。
- Delta spec 不携带校验元数据（仅主 spec 需要）。

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
