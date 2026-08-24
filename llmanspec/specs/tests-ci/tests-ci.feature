# language: zh-CN
# capability: tests-ci
# purpose: 规范 CI 质量门与必需的校验检查。
# scope: llmanspec/specs/tests-ci

功能: tests-ci

  @req:r35 @human
  场景: CI quality gates on locked nightly
    - CI on main MUST run the check job on the repository-locked nightly baseline executing `just check-all` (or an equivalent nightly-based sequence), MUST run the build job release build on that same baseline, and MUST keep test code free of clippy warnings such as `len_zero` under `cargo +nightly clippy -- -D warnings`.

  @req:r68 @human
  场景: check-all schema gate
    - `just check-all` MUST execute `just check-schemas` so generated JSON schemas and sample configs remain valid and usable.
