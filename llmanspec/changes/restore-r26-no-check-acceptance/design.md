# Design: restore-r26-no-check-acceptance

## 决策

纯召回,不引入新设计:逐字恢复 v0.0.66 `validate-check.feature` 中的场景「BDD-on 时 validate --no-check 跳过 runner」,仅做两处单轨格式适配:

1. tag 从旧的 `@executable` 单行补挂 `@req:r26`(单轨要求验收场景挂回规则;旧文件该场景恰好缺 req 标签);
2. 步骤文本(假如/当/那么)与断言口径(退出码为零、stderr 不含 `BDD check failed`)保持原文,复用 `tests/bdd_steps.rs` 既有 step,不新增 step、不改代码。

## 备选与否决

- 同时召回「BDD-off 时 validate --check 不执行 runner」:其行为已被现有场景「BDD-off 时 validate 静默忽略 .feature 文件」(`--strict --no-check`)+ r26 @human 覆盖,超出本次薄弱点范围,不做。
- 改写断言为 JSON 输出断言:原口径(human-readable stderr)即可被既有 step 驱动,保持最小 diff。

## 风险

无 `@human` 文本改动,不触发锁定规则门禁;BDD 场景数 48→49,不影响其他 capability。
