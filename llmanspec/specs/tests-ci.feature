# language: zh-CN
# capability: tests-ci
# purpose: 规范 CI 质量门与必需的校验检查。
# scope: llmanspec/specs/tests-ci.feature

功能: tests-ci

  @req:r35
  规则: CI quality gates on locked nightly
    CI on main MUST run the check job on the repository-locked nightly baseline executing `just check-all` (or an equivalent nightly-based sequence), MUST run the build job release build on that same baseline, and MUST keep test code free of clippy warnings such as `len_zero` under `cargo +nightly clippy -- -D warnings`.
  @req:r68
  规则: check-all schema gate
    `just check-all` MUST execute `just check-schemas` so generated JSON schemas and sample configs remain valid and usable.
  @req:r59
  规则: Gherkin BDD 套件绑定可执行场景
    仓库 MUST 维护 tests/bdd 下的 Gherkin BDD 套件（bun 桥，零 npm 依赖）：以受控环境（隔离 HOME 与 LLMAN_CONFIG_DIR、PATH 剔除宿主插件）驱动构建出的 llman 二进制子进程，执行 llmanspec/specs 中已绑定步骤定义的嵌套场景；未绑定 feature MUST NOT 被执行（以 allowlist 显式声明）。just bdd MUST 先构建调试二进制再运行套件，并接入 check-all 与 CI。


    场景: bdd-suite-runs-bound-scenarios-green
      假如 llman 二进制已构建
      当 运行 bun test tests/bdd
      那么 退出码为零