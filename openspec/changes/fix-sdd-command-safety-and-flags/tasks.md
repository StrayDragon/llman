## 1. Safety & Correctness（必须完成）
- [ ] 1.1 标识符校验：对 change/spec ID 增加统一校验（拒绝 `/`、`\\`、`..` 等），用于所有 path join 的入口。
  - 验证：对非法 ID 运行 `show/validate/archive` 均返回错误且不产生任何文件系统写入。
- [ ] 1.2 `archive` 安全：在解析 change id 后，仅允许在 `llmanspec/changes/<id>` 范围内移动目录，禁止越界。
  - 验证：构造包含分隔符的输入，确认不会触及 `llmanspec/` 之外路径。
- [ ] 1.3 `list` 冲突策略：`--specs` 与 `--changes` 互斥，冲突时报错并退出非零。
  - 验证：`llman sdd list --specs --changes` 返回清晰错误；单独使用任一 flag 行为不变。
- [ ] 1.4 `update-skills` multi-tool + `--path`：显式拒绝并提示推荐用法（按 tool 分开执行或提供更安全的替代）。
  - 验证：`--all --path <p>` 返回错误且不会产生部分写入；单 tool + `--path` 行为保持可用。

## 2. Behavior Parity（保持语义不变）
- [ ] 2.1 `validate` 语义保持：缺少 delta 仍为 Error；本变更不引入“非 strict 降级为 warning”的新语义，仅增强安全与边界提示。
  - 验证：无 delta 的 change 仍失败，并包含可执行修复提示。
- [ ] 2.2 base ref 解析增强：当 `origin/main|origin/master` 不存在时，优先尝试本地 `main|master`，再提示用户设置 `LLMANSPEC_BASE_REF`。
  - 验证：无 remote 的 repo 也能得到稳定的 base ref 或清晰提示。

## 3. Performance（实现优化）
- [ ] 3.1 批量校验缓存：在一次 `validate --all/--specs` 中复用共享 git 结果（base ref、merge-base、dirty、diff names），减少子进程数量。
  - 验证：在包含多个 specs 的项目中，git 子进程数量显著下降且 staleness 结论不变。

## 4. Tests（可回归）
- [ ] 4.1 测试：ID 校验（拒绝 `../`、包含分隔符的 ID）。
- [ ] 4.2 测试：`list --specs --changes` 冲突必报错。
- [ ] 4.3 测试：`update-skills` multi-tool + `--path` 明确拒绝且不写入。
- [ ] 4.4（可选）测试：staleness 缓存不改变结果（对固定 repo fixture 的输出一致）。

## 5. Acceptance（验收标准）
- [ ] 5.1 新增规范场景全部可通过测试或可重复手工步骤验证。
- [ ] 5.2 不引入 `llmanspec/` 目录以外的写入或移动行为。
- [ ] 5.3 性能优化不改变校验结果（仅减少 git 调用）。

## 6. Validation
- [ ] 6.1 `openspec validate fix-sdd-command-safety-and-flags --strict --no-interactive`
- [ ] 6.2 `just test`
