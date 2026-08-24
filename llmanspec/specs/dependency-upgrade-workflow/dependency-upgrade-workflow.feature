# language: zh-CN
# capability: dependency-upgrade-workflow
# purpose: 规范依赖升级工作流：以 lockfile 优先的顺序进行依赖升级。
# scope: llmanspec/specs/dependency-upgrade-workflow

功能: dependency-upgrade-workflow

  @req:r21 @human
  场景: lockfile-first dependency upgrade
    - Maintainers MUST upgrade dependencies in lockfile-first order on the locked nightly baseline: update the lockfile and run validation before changing dependency version constraints in the manifest; ONLY when a lockfile-only update is insufficient to build or pass checks MAY they apply the minimal required manifest constraint boundary change and re-validate on nightly.

  @req:r52 @human
  场景: upgrade outcome verifiable and reversible
    - When manifest dependency constraints change, the change MUST be minimal and limited to compatibility needs and MUST pass project quality gates. After an upgrade batch, maintainers MUST be able to show nightly-based validation passed and MUST retain a rollback path to the previous known-good lock state.
