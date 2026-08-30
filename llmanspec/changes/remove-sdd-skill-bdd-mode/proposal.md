---
depends_on: []
rules_edit_acked: true
---

# 退役托管 skill 的 bdd_mode 元数据与一致性门禁

## Why

`metadata.llman_sdd.bdd_mode` 是双轨时代（BDD-on/off 分叉管线）的产物：标记
skill 渲染产物应遵循哪套管线叙事。统一 Git-native 单轨后（r61 不再区分命令
分叉），模板中的 bdd 条件渲染只剩 2 处且都属于 **runner**（validate skill 的
`{% if bdd_enabled %}`、verify 的 `{% if bdd_verify_prompt %}`）——管线叙事
本身已无 on/off 之分，`bdd_mode` 只剩「校验 skill 与 config 是否一致」的自证
用途（r95 门禁），每 skill 每次渲染白付一行 + 一套门禁机器
（skill_consistency 的 config 比对、validate/init 双路检查）。

用户决策：feature-as-spec 已是唯一主载体（features 首位），退役 bdd_mode
及其配置处理机器；**`bdd:` 配置段（runner）保留**——它是 `validate --check`
执行 GWT 场景的引信，与模式分叉无关。

## What Changes

- 18 个模板（双 locale）frontmatter 删除 `bdd_mode: "{{ bdd_mode }}"` 行；
  `build_template_vars` 移除 bdd_mode 变量。
- `skill_consistency.rs`：移除 `expected_bdd_mode` 与 bdd_mode 解析/比对；
  `metadata.llman_sdd` 保留（`skill_set` 枚举门禁照旧）。
- r95 修订（锁定规则，ack 已带）：bdd_mode 要求与一致性比对删除；缺失
  llman_sdd / skill_set 非法的门禁保留；validate 与 init --update 双路检查
  口径不变（MUST NOT 因无前缀自定义 skill 失败照旧）。
- r95 的 2 个 @executable 场景随之改写：场景 1（拒绝缺失 llman_sdd 元信息）
  保留原语义；场景 2 改写为「init --update 写入 llman_sdd 元信息后 validate
  通过」（不再提 bdd_mode）。
- 测试侧（apply 内）：bdd_steps fixture 与 it 测试中对 bdd_mode 的断言/
  写入同步适配。

## Capabilities

- `sdd-workflow`：r95 修订 + 场景改写（锁定规则，ack）。

## Impact

- 受影响范围：36 处模板 frontmatter、skill_consistency/templates 代码、
  渲染产物 resync、bdd_steps/it 测试、1 条 live 规则 + 2 个场景。
- 行为变化：skill frontmatter 少一行；validate/init 不再比对 bdd_mode；
  `bdd:` 段与 `validate --check` runner 语义完全不变。
- 不做：`bdd:` 段退役（用户确认保留）；skill_set 字段改动；运行时元 skill。
