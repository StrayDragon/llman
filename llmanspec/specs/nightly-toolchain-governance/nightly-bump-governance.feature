# language: zh-CN
# 对应 spec: nightly-toolchain-governance — 升级锁定的 nightly 日期时 MUST 经项目
# 质量门验证；且 MUST 保留回退到上一已知良好 nightly 的文档化路径。
功能: nightly 升级经显式门禁且可回退
  @req:r1
  场景: 升级 nightly 日期须通过质量门
    假如 {toolchain_file} 被改为更新的 nightly 日期
    当 维护者评估该 bump
    那么 改动通过基于 nightly 的格式、lint、测试与 release 构建检查

  @req:r1
  场景: 新 nightly 引入阻断性回归时可回退
    假如 nightly 升级后出现阻断合并或发布的回归
    当 维护者回退
    那么 可恢复先前的锁定 nightly 基线
    而且 无需重写无关代码即可恢复绿色构建
