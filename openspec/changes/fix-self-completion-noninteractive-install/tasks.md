## 1. Implementation
- [ ] 1.1 completion install 增加 `--yes`（可选 `-y`），仅作用于 `--install` 路径。
  - 验证：`llman self completion --shell bash --install --yes` 在无 TTY 时仍可成功写入。
- [ ] 1.2 非交互拒绝：非交互环境下 `--install` 未传 `--yes` 时必须返回错误且不写入。
  - 验证：重定向 stdin 时运行，确认 rc/profile 文件未改变。
- [ ] 1.3 交互默认确认：交互环境下不传 `--yes` 仍提示确认；传 `--yes` 则跳过提示。
  - 验证：两种路径均保持幂等更新 marker block。

## 2. Tests
- [ ] 2.1 测试：`--install --yes` 路径可在无 TTY 下更新 marker block。
- [ ] 2.2 测试：非交互 + 未传 `--yes` 必须拒绝写入。

## 3. Acceptance
- [ ] 3.1 默认交互行为不变（仍需确认）。
- [ ] 3.2 非交互必须显式 `--yes` 才能写入，避免脚本误写。

## 4. Validation
- [ ] 4.1 `openspec validate fix-self-completion-noninteractive-install --strict --no-interactive`
- [ ] 4.2 `just test`
