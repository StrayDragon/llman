# Design: restore-truncated-rule-statements

## 根因链

1. BDD-pilot 时代每个旧 `.feature` 的规范陈述写在多行 `# 对应 spec:` 头注释里(行断点可落在句中,如括号内的列表中间)。
2. 后续生成 `spec.toon` 时 statement 只取了头注释**第一行**——首行断在句中的陈述尾部丢失(例:r44 丢 `DYLD_*/PATH 及大小写变体）,拒绝时不启动 codex;import 交互式创建 provider;主命令/run 支持 -- 透传;account 提供 edit 与 import。`)。
3. 单轨迁移(5f68096)按「statement 原文入描述」忠实搬运残缺文本 → 主线今日 17 条断尾。

## 恢复方法(可复核)

- 语料:`git show a275d6a:llmanspec/specs/<cap>/<file>.feature` 的全部 `#` 行(除 `# language:`)按原顺序拼接 = 完整陈述。
- 判定:主线现陈述(剥迁移期包装后)是完整陈述的**规范化真前缀**且尾部损失 ≥6 字符 → 断尾。
- 替换:按 (cap, req) 定位 `@human` 场景,把描述行替换为完整文本(单行 bullet);`The system MUST satisfy the harness scenarios for ...` 包装保留在前。
- 验收:替换后重跑同一前缀扫描,命中数必须为 0。

## 否决的备选

- 同时恢复 toon 注记/GWT 行:已另行审计(53 行中 40 条为指向已删文件的指针噪声、13 条为主线陈述/executable 覆盖或已过时),不迁回。
- 深挖 pre-v0.0.65(ed62e51 压缩前)历史:该压缩是经审阅的刻意决策,超出本次迁移损失审计边界。

## 风险

- 17 条均为锁定 @human 规则,编辑需 `rules_edit_acked: true`(已设)。
- 替换是机械的(前缀断言通过才写),不含语义改写;若某条陈述主线已有演化痕迹则前缀不匹配、不会误替换。
