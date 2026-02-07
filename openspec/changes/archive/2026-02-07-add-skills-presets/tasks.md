# Tasks: add-skills-presets

## 1. 数据结构与运行时预设

- [x] 1.1 在 `src/skills/catalog/registry.rs` 扩展 `Registry.presets` 与 `PresetEntry`
- [x] 1.2 保持 serde 向后兼容（无 `presets` 时可正常加载）
- [x] 1.3 实现运行时预设构建：优先使用 `registry.presets`，为空时按 `<preset>.<skill>` 自动推断
- [x] 1.4 约束自动推断仅运行时生效，不写回 `registry.json`

## 2. 预设解析与 fail-fast 校验

- [x] 2.1 实现预设继承解析（`extends`）和去重合并
- [x] 2.2 实现循环依赖检测
- [x] 2.3 实现预设引用目录存在性检查（`skill_dirs` 必须可映射到已扫描技能）
- [x] 2.4 实现空预设检查（解析后技能集合不能为空）
- [x] 2.5 在 `llman skills` 进入交互前执行全部校验并报错中止

## 3. 交互流程调整（无新增 CLI 参数）

- [x] 3.1 在交互入口新增模式选择：`Apply preset` / `Select individually` / `Exit`
- [x] 3.2 实现 `Apply preset` 流程：预设选择 -> agent/scope -> 确认应用
- [x] 3.3 保持 `Select individually` 的既有行为与同步语义
- [x] 3.4 不新增任何 presets 相关 CLI 参数

## 4. 分组展示与体验

- [x] 4.1 按目录名分组展示技能列表（`group.name` -> `group`，其余归入 `ungrouped`）
- [x] 4.2 在交互展示中标识分组，降低长列表选择成本

## 5. 测试与文案

- [x] 5.1 单元测试：预设推断、继承解析、循环检测、缺失引用报错、空预设报错
- [x] 5.2 交互路径测试：存在预设时可走 `Apply preset`，无预设时隐藏该入口
- [x] 5.3 更新 i18n 字符串：模式菜单、预设错误、校验失败提示

## Verification Checklist

- [x] `openspec validate add-skills-presets --strict --no-interactive`
- [x] `cargo +nightly fmt -- --check`
- [x] `cargo +nightly clippy --all-targets --all-features -- -D warnings`
- [x] `cargo +nightly test --all`
