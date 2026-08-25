# Tasks: restore-truncated-rule-statements

## 测试边界(seam)

- 无代码 seam;断言面 = 文本恢复后断尾前缀扫描归零 + 既有门禁全绿(validate --all strict / BDD 不受陈述文本影响)

## 垂直切片

### t1: Specs landing——17 条陈述机械恢复
- [x] 绑定分支上按 (cap, req) 用旧头注释全行拼接文本替换断尾描述行(带断言:现文本必须是新文本规范化前缀,否则中止)

### t2: 验收
- [x] 断尾前缀复扫命中 0
- [x] `llman sdd validate --all --strict` 全绿
- [x] `cargo +nightly test --features bdd` 50 场景全绿
