---
depends_on: []
branch: sdd/add-executable-acceptance-batch1
base_branch: main
base_sha: 22072f5faf9e813641f8de7aba1a06bbb1fffb1d
---

## Why

llman-sdd 0.5 升级后，specs 迁移为原生分层格式，review 计量出 97 条裸规则（无嵌套可执行验收的规则）。本 change 是压降 backlog 的第一批：为 CLI 可观测、可受控执行的 7 条裸规则补写嵌套验收场景并绑定进 tests/bdd 套件，使规格从"文档"向"可执行规格"渐进靠拢。

## What Changes

- config-paths：r17（配置目录优先级）、r4（tilde 展开与非法路径）、r72（非法路径报错）补可执行验收，经 `llman --print-config-dir-path` 受控驱动。
- config-schemas：r18（schema apply root discovery）、r49（schema generate）、r73（首次运行样例生成与 schema 校验）补可执行验收，经 `llman self schema generate/apply/check` 受控驱动。
- cli：r13（配置守卫范围与命令结构）补命令面验收，经 `llman sdd spec --help` 委托面断言。
- tests/bdd：新增 config_paths.ts / config_schemas 扩展步骤与 allowlist 注册。
- 明确不在本批：cli r8/r9/r10 与 config-paths r46（context/index 能力已外置为 llman-context 插件，需 embedding API，非受控可执行；其外置化契约重写留待后续 change）。
