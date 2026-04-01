<!-- llman-template-version: 1 -->
## Canonical YAML Spec Contract（experimental）

当项目配置 `spec_style: yaml` 时，SDD 主 spec 与 delta spec 都必须以 **单个** fenced ` ```yaml ` code block 承载一份 canonical YAML 文档。

### Main spec（`llmanspec/specs/<feature-id>/spec.md`）

```yaml
kind: llman.sdd.spec
name: sample
purpose: 'One-line overview.'
requirements:
- req_id: r1
  title: Requirement title
  statement: System MUST do something.
scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）

```yaml
kind: llman.sdd.delta
ops:
- op: add_requirement
  req_id: r1
  title: New requirement
  statement: System MUST do the new thing.
  from: null
  to: null
  name: null
op_scenarios:
- req_id: r1
  id: happy
  given: ''
  when: the new trigger happens
  then: the new outcome is observed
```

### 备注
- YAML 写回会尽量使用保留注释/格式的 overlay 更新。
- 当 lossless 写回无法应用时，会回退为仅重写 fenced YAML payload 的确定性输出（payload 内的注释可能丢失）。
- 同一项目内不允许混用多种风格；跨风格迁移必须使用 `llman sdd convert`。
