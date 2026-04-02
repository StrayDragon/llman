<!-- llman-template-version: 1 -->
常见校验修复（YAML 风格）：

1) Main spec 缺少 YAML frontmatter（仅 main spec 需要）：
```markdown
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - llman sdd validate <feature-id> --type spec --strict --no-interactive
llman_spec_evidence:
  - <evidence>
---
```

2) Main spec 缺少 canonical ` ```yaml ` payload（`llmanspec/specs/<feature-id>/spec.md`）：
```yaml
kind: llman.sdd.spec
name: sample
purpose: 'One-line overview.'
requirements:
- req_id: r1
  title: Title
  statement: System MUST do something.
scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

3) Change 缺少 delta ops：至少补一个 op + scenario（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）：
```yaml
kind: llman.sdd.delta
ops:
- op: add_requirement
  req_id: r1
  title: Title
  statement: System MUST do something.
  from: null
  to: null
  name: null
op_scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

备注：
- `yaml` 文件必须只有一个 ` ```yaml ` fence。
- YAML 写回会尽量使用保留注释/格式的 overlay 更新；失败时会回退为确定性重写 fenced YAML payload（payload 内注释可能丢失）。
- `yaml` 为 experimental：跨风格迁移请使用显式的 `llman sdd convert`。
