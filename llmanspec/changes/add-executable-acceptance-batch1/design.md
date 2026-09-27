# Design

## 验收驱动方式

- config-paths 三条规则全部经 `llman --print-config-dir-path` 断言（src/cli.rs 直接解析并退出，无副作用），优先级/tilde/非法路径均可从 stdout/stderr 精确断言。
- config-schemas 三条规则经 `llman self schema generate/apply/check`：generate/apply 写入 fixture 项目与配置目录；check 对已生成的 schema 与样例做校验。root discovery 断言在项目子目录执行时仍定位项目根。
- cli r13 经 `llman sdd spec --help` 委托面断言 add-req 等统一命令名存在。

## 步骤与夹具

- tests/bdd/steps/config_paths.ts：print-config-dir-path 的三种优先级组合、tilde 展开（HOME 指向临时目录）、空/空白路径报错。
- tests/bdd/steps/config_schemas.ts 扩展：generate/apply/check 步骤，fixture 项目布局（llmanspec/ + 顶层 config.yaml）。
- allowlist 增加 config-paths.feature；cli.feature 已全量。

## 兼容性

- 不改任何既有断言；纯新增嵌套场景与步骤。
- cli r8/r9/r10、config-paths r46 保持裸规则（外置化遗留，另开 change 处理其契约重写）。
