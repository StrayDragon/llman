## 1. Implementation
- [x] 1.1 扩展 skills 配置解析：为 target 增加 `mode` 字段（link/copy/skip），默认 link，并更新相关类型与测试。
- [x] 1.2 更新同步与目标应用逻辑：来源只读；target 根据 mode 执行 link/copy/skip。
- [x] 1.3 实现 copy 模式元数据写入与冲突检测（`.llman-skill.json`）。
- [x] 1.4 移除 `--relink-sources` 参数与相关确认逻辑，更新 CLI 帮助与错误提示。
- [x] 1.5 新增冲突处理参数 `--target-conflict=overwrite|skip`，交互模式提示选择，非交互无参数时报错并提示。
- [x] 1.6 更新交互管理器：skip 目标只读展示；copy 目标启用/禁用按新逻辑处理。
- [x] 1.7 更新 i18n 文案与错误提示，补充/调整测试用例。

## 2. Verification
- [x] 2.1 `just test`
- [ ] 2.2 交互验证：`llman skills`（无 flags）
- [x] 2.3 非交互验证：`llman skills --target-conflict=skip`
