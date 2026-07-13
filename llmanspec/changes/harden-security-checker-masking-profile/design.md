# Design: harden-security-checker-masking-profile

## SecurityChecker fail-closed

- Keep printing warnings to stderr for diagnosis.
- After non-empty warnings：`bail!` / non-zero，不调用 `Command::status`。
- 不引入 `--force-insecure`（除非未来单独 change）；本 change 一律 fail-closed。

## Masking

- Centralize sensitive-key predicate（大小写不敏感 `contains`）：
  `KEY|TOKEN|SECRET|PASSWORD|PASSWD|CREDENTIAL`
- Reuse in `get_display_vars`.

## PROFILE home jail

- Resolve `PROFILE` to absolute path；require `starts_with(home)`（canonicalize when possible）。
- Reject otherwise with clear error；default path unchanged.
