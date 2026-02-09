## 1. Nightly baseline alignment

- [x] 1.1 Update `rust-toolchain.toml` to the target pinned nightly date.
- [x] 1.2 Align `.github/workflows/ci.yaml` jobs to use the same pinned nightly baseline.
- [x] 1.3 Remove or rewrite stale toolchain comments so repository docs/config no longer imply stable CI.

## 2. Dependency upgrade execution

- [x] 2.1 Perform lockfile-first dependency update under the pinned nightly baseline.
- [x] 2.2 Apply minimal `Cargo.toml` version-bound updates only where lockfile-only updates are insufficient.
- [x] 2.3 Verify upgrade batch with nightly quality gates (`fmt`, `clippy -D warnings`, `test`, `build-release`).

## 3. Validation and rollback readiness

- [x] 3.1 Run CLI smoke checks for key flows (`llman x cc`, `llman x codex`, `llman x cursor`, `llman prompt`, `llman tool`).
- [x] 3.2 Document rollback steps to the previous known-good nightly and lock state in change notes/PR description.
- [x] 3.3 Record final verification outcomes and changed dependency/toolchain scope for reviewer audit.
