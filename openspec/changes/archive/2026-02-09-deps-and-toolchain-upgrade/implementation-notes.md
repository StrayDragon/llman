## Implementation Notes (2026-02-09)

### Rollback Plan

If this upgrade introduces blocking regressions, rollback in this order:

1. Restore `rust-toolchain.toml` to the previous known-good pinned nightly:
   - from `nightly-2026-02-08`
   - back to `nightly-2026-01-25`
2. Restore `Cargo.lock` to the previous known-good version from the parent commit (or last green CI commit).
3. If needed, revert CI toolchain alignment in `.github/workflows/ci.yaml` together with the toolchain rollback to keep local/CI parity.
4. Re-run:
   - `cargo +nightly-2026-01-25 fmt --all -- --check`
   - `cargo +nightly-2026-01-25 clippy --all-targets --all-features -- -D warnings`
   - `cargo +nightly-2026-01-25 test --all`
   - `cargo +nightly-2026-01-25 build --release`

### Verification Outcomes

All checks passed with proxy-enabled network (`HTTPS_PROXY=http://127.0.0.1:20171`) on pinned nightly `nightly-2026-02-08`:

- Quality gates:
  - `cargo +nightly-2026-02-08 fmt --all -- --check` ✅
  - `cargo +nightly-2026-02-08 clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo +nightly-2026-02-08 test --all` ✅
  - `cargo +nightly-2026-02-08 build --release` ✅
- CLI smoke checks (with `LLMAN_CONFIG_DIR=./artifacts/testing_config_home`):
  - `./target/release/llman x cc --help` ✅
  - `./target/release/llman x codex --help` ✅
  - `./target/release/llman x cursor --help` ✅
  - `./target/release/llman prompt --help` ✅
  - `./target/release/llman tool --help` ✅

### Changed Scope for Review

- Toolchain baseline:
  - `rust-toolchain.toml`: pinned nightly updated to `nightly-2026-02-08`.
- CI alignment:
  - `.github/workflows/ci.yaml`: jobs now install toolchain via shared env `RUST_TOOLCHAIN: nightly-2026-02-08`.
- Dependency update:
  - Lockfile-first upgrade completed via `cargo +nightly-2026-02-08 update`.
  - `Cargo.lock` updated (55 packages advanced to latest Rust 1.95.0-nightly-compatible versions).
- Manifest bounds:
  - No dependency version-bound updates in `Cargo.toml` were required for this batch.
  - `Cargo.toml` change in this batch is comment cleanup only (toolchain policy wording).
