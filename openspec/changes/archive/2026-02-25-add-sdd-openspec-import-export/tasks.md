## 1. CLI 命令面与参数约束

- [x] 1.1 在 `src/sdd/command.rs` 新增 `import` 与 `export` 子命令，并要求 `--style` 必填。  
- [x] 1.2 将 `--style` 校验限制为 `openspec`，非法值返回非零错误。  
- [x] 1.3 更新 `llman sdd --help` 输出与相关说明，确保不暴露 `migrate` 入口。  

## 2. 迁移计划构建与路径安全

- [x] 2.1 新增互转执行模块（建议 `src/sdd/project/interop.rs`），实现 source/target 发现、计划构建与 dry-run 输出。  
- [x] 2.2 实现完整迁移范围扫描：`specs`、active `changes`、`changes/archive`。  
- [x] 2.3 实现冲突检测（目标同名文件即失败）与路径边界保护（禁止越界写入）。  
- [x] 2.4 检测非标准目录（如 `openspec/explorations/`），输出 warning 并复制到目标侧。  

## 3. 执行门禁（默认演练 + 双确认）

- [x] 3.1 所有 `import/export` 命令默认先输出 dry-run 计划。  
- [x] 3.2 在交互终端接入两次确认（`Confirm` + 确认短语）后再执行写入。  
- [x] 3.3 在非交互环境拒绝写入并返回非零，确保仅可演练不可执行。  
- [x] 3.4 迁移成功后在交互模式提示是否删除旧迁移目录，默认不删除。  

## 4. 双向内容映射与元数据补齐

- [x] 4.1 实现 `import --style openspec`：`openspec/` -> `llmanspec/` 文件映射。  
- [x] 4.2 实现 `export --style openspec`：`llmanspec/` -> `openspec/` 文件映射。  
- [x] 4.3 在 export 方向自动补齐 `openspec/config.yaml` 与 active change `.openspec.yaml`。  
- [x] 4.4 在 import 方向为缺失 frontmatter 的 spec 自动补齐 llman 必需字段。  

## 5. i18n、测试与规范同步

- [x] 5.1 新增 `sdd.import.*` / `sdd.export.*` 文案键并接入命令输出。  
- [x] 5.2 为双向迁移、冲突失败、非交互拒绝、非标准目录 warning+复制、旧目录删除确认（默认否）、元数据补齐新增集成测试。  
- [x] 5.3 更新 `openspec/specs/sdd-workflow/spec.md` 主规范，纳入 `import/export` 与安全执行模型。  
- [x] 5.4 运行 `openspec validate add-sdd-openspec-import-export --strict --no-interactive` 并修复校验问题。  
