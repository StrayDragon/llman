## 1. Spec updates
- [x] 1.1 更新 `sdd-workflow`：模板版本元信息写入 `metadata` 字段

## 2. Implementation
- [x] 2.1 更新 SDD skills 模板：`llman-template-version` 移入 `metadata`
- [x] 2.2 更新模板检查脚本：从 `metadata` 读取版本并继续校验一致性

## 3. Verification
- [x] 3.1 `just check-sdd-templates`
