# Design

## 四种驱动方式与步骤归属

1. repo.ts：Given '本仓库真实工作区'（项目根 = REPO_ROOT），复用相对路径/内容包含断言。适用 tests-ci/nightly/dependency/sdd-workflow r28。
2. skills.ts：Given '已启用可选技能 {name} 的 sdd 项目'（extra_skills + init --update）；断言 '渲染技能 {name} 内容包含 {text}' / '渲染技能 {name} 存在'。适用 sdd-structured 9 条、sdd-workflow r100-r108、prompts r82、compaction r64。
3. tools.ts：tool 四件套的文件夹具（python 文件/无用目录/ignore 文件/AGENTS.md）与运行步骤。
4. sdd.ts 扩展：dirty-tree Given、worktree 步骤、嵌套 change fixture、frontmatter 未知字段 fixture、graph/config/context/index 运行词表（多为通用 args 覆盖）。

## 已知不可执行项的处理

无——本批后所有规则均有嵌套场景。交互固有的流程（如 skills 两段式多选、prompts 编排）以非交互契约面（拒绝执行/帮助面/列表输出）作为可执行验收。

## 外置化契约重写

- cli r8/r9/r10、config-paths r46：context/index 能力自 0.0.79 起经 llman-* 插件外置；规则描述改为委托现实，场景断言插件缺席时的干净报错与无副作用。
- sdd-openspec-interop r30/r63：上游 0.5 未提供 import/export 命令面；规则改写为现状契约，场景断言 unknown command。
