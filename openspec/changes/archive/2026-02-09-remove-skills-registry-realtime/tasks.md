## 1. 规范与接口收敛

- [x] 1.1 更新 `skills-management` 主规范对应实现注释与文案：移除所有 registry 持久化语义，统一为实时计算语义。
- [x] 1.2 调整 `SkillsPaths` 与相关配置解析接口，移除 `registry_path` 字段及其依赖调用点。

## 2. 移除 registry 读写链路

- [x] 2.1 从 `src/skills/cli/command.rs` 中移除 `Registry::load/save`、`update_registry_for_target`、`ensure_target_defaults` 等依赖状态持久化的流程。
- [x] 2.2 重写非交互路径的 desired 计算：以 `is_skill_linked` 为事实来源，未链接项回退 `target.enabled`，并复用既有冲突处理策略。
- [x] 2.3 清理 `src/skills/catalog/registry.rs` 与相关导出/引用（删除模块或降级为过渡占位，确保无死代码警告）。

## 3. 预设逻辑简化为目录推断

- [x] 3.1 调整 runtime preset 构建逻辑：删除 `registry.presets` 分支，仅保留目录名推断路径。
- [x] 3.2 移除 `extends/skill_dirs` 校验与解析流程，保持分组树、三态选择、去重与搜索过滤行为不变。

## 4. 测试与文档更新

- [x] 4.1 更新/替换依赖 `registry.json` 的单元测试与集成测试，新增“非交互实时计算”场景覆盖（已链接优先、未链接回退 enabled）。
- [x] 4.2 更新 `docs/skills-registry-presets.md`、`README.md` 等文档，明确 `registry.json` 废弃与目录命名迁移方式。
- [x] 4.3 使用测试配置目录执行验证命令（至少覆盖 `just test` 或等价最小测试集），确保无回归且无新增警告。
