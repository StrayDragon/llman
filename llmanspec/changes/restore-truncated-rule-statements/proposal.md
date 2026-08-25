---
depends_on: []
rules_edit_acked: true
---

## Why

全量审计 a275d6a(迁移前旧状态) vs 主线发现真实数据损伤:旧 `.feature` 头注释(`# 对应 spec:`)是**多行**陈述,而 toon 生成时只取了**第一行**;第一行断在句中的(如 `（LD_PRELOAD/LD_LIBRARY_PATH/` 后换行),尾部规范内容在写入 toon 时丢失,单轨迁移又忠实搬运了残缺文本——**主线今日有 17 条 @human 陈述断尾**(跨 13 个 capability 文件),丢失的是活的 MUST 内容(危险键清单、子命令面、union targets、dry-run 双确认语义等)。审计方法:412 条旧场景按 @req 归组(105 组,0 req 丢失)逐组人工复核 + 头注释前缀匹配扫描。

## What Changes

- 用旧 `.feature` 头注释全行拼接文本,机械恢复 17 条断尾陈述(替换各文件中对应 `@req:<id> @human` 场景的描述行;带 `The system MUST satisfy the harness scenarios for` 包装的保留包装):
  - claude-code-account-management r11;cli-experience r43;codex-account-management r15/r44;codex-agents-management r16;config-schemas r18;cursor-claude-ignore-sync r19/r50/r74;sdd-ab-evaluation r56;sdd-eval-acp-pipeline r28;sdd-eval-workflow-dsl r29;sdd-openspec-interop r30;sdd-specs-compaction-guidance r64;sdd-template-units-and-jinja r66;skills-management r34/r84
- 无行为面变化(纯陈述文本恢复);锁定规则编辑已 ack。

## Capabilities

- 上述 13 个 capability 的既有 requirement 陈述恢复(不新增/删除 req)

## Impact

- 仅 live spec 文本;BDD/测试不受影响(陈述不参与 step 绑定)。恢复后断尾复扫应为 0。
