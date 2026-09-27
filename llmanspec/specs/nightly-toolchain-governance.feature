# language: zh-CN
# capability: nightly-toolchain-governance
# purpose: 规范 nightly 工具链治理：锁定 nightly 作为唯一构建基线。
# scope: llmanspec/specs/nightly-toolchain-governance.feature

功能: nightly-toolchain-governance

  @req:r23
  规则: nightly 升级经显式门禁且可回退
    对应 spec: nightly-toolchain-governance — 升级锁定的 nightly 日期时 MUST 经项目 质量门验证；且 MUST 保留回退到上一已知良好 nightly 的文档化路径。

    场景: nightly-upgrade-gate-documented
      假如 本仓库真实工作区
      那么 相对路径 .github/workflows/ci.yaml 内容包含 just check-all
  @req:r54
  规则: 锁定 nightly 作为单一构建基线
    The repository MUST define a single locked nightly baseline in {toolchain_file} as the authoritative local build toolchain, and developer build/check commands MUST resolve to that baseline.

    场景: single-locked-nightly-baseline
      假如 本仓库真实工作区
      那么 相对路径 rust-toolchain.toml 内容包含 channel
