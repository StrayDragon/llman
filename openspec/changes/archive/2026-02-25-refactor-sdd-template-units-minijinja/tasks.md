## 1. 渲染与单元注册基础设施

- [x] 1.1 设计并实现 SDD 模板单元注册结构（locale 维度 + unit id）及加载接口。
- [x] 1.2 在 `src/sdd/project/templates.rs` 引入 MiniJinja 渲染流程，支持模板注入单元并替代现有 region 展开路径。
- [x] 1.3 实现缺失单元、重复单元、缺失渲染变量的明确错误处理，并补充对应单元测试。

## 2. 模板资源拆分与迁移

- [x] 2.1 将共享结构化协议、future 规划、archive/freeze 指导等可复用片段拆分为独立模板单元文件（en 与 zh-Hans 对齐）。
- [x] 2.2 将 `templates/sdd/{locale}/skills/*.md` 迁移为 MiniJinja 单元注入写法，移除 `{{region: ...}}` 依赖。
- [x] 2.3 将 `templates/sdd/{locale}/spec-driven/*.md` 迁移为 MiniJinja 单元注入写法，确保渲染后产物保持自包含。

## 3. 兼容性与回归验证

- [x] 3.1 更新 `src/sdd/project/templates.rs` 的 embedded 模板映射与加载逻辑，保证新单元文件可被打包与读取。
- [x] 3.2 补充/更新集成测试，验证 `update-skills` 产物包含 archive freeze/thaw 与 future-to-execution 注入内容。
- [x] 3.3 运行并修复质量门禁：`just check-sdd-templates`、`cargo +nightly test --test sdd_integration_tests -q`、`openspec validate refactor-sdd-template-units-minijinja --type change --strict --no-interactive`。
