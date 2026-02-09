## Context

当前仓库通过 `rust-toolchain.toml` 固定 nightly，但 CI 工作流仍显式安装 stable，且 `Cargo.toml` 注释也描述为“edition 2024 requires nightly / CI uses nightly”，与实际执行路径不一致。与此同时，依赖存在可升级版本，若不建立固定流程，后续会持续累积“工具链漂移 + 锁文件滞后”的维护成本。

该变更的核心不是引入 stable 兼容路径，而是在 **nightly-only** 前提下，让工具链升级与依赖升级可重复、可回滚、可验证。

## Goals / Non-Goals

**Goals:**
- 将 nightly 固定版本作为唯一构建基线，并在本地与 CI 上一致执行。
- 建立依赖升级流程：先锁文件升级，再按需调整清单约束，最后通过 nightly 质量门禁验证。
- 让升级步骤具备可审计性（明确命令、验证项、失败回滚策略）。
- 不改变 CLI 的外部功能行为，仅改善构建与维护质量。

**Non-Goals:**
- 不新增 stable 主支持路径，也不承诺 dual-toolchain 兼容。
- 不在本次变更中进行大版本重构或跨模块架构重写。
- 不扩展新的平台承诺（Windows 仍非主要目标）。

## Decisions

### Decision 1: Keep a pinned nightly baseline
项目继续使用按日期固定的 nightly（`nightly-YYYY-MM-DD`）作为基线，而非“floating nightly”。

**Rationale:**
- 避免每日 nightly 漂移导致不可复现构建。
- 在获得新特性的同时，保留可定位问题的能力（可精确回退到上一固定日期）。

**Alternatives considered:**
- Floating nightly：更新快，但回归不可预测且难以回溯。
- Stable-only：与本次明确策略冲突，不采用。

### Decision 2: Align CI to the same pinned nightly
CI 检查与发布构建应显式使用与仓库一致的 nightly 基线（或从同一来源解析），不得默认 stable。

**Rationale:**
- 防止“本地 nightly 通过、CI stable 通过/失败”这类信号失真。
- 提高 `just check-all` 与 CI 结果的一致性，减少排障成本。

**Alternatives considered:**
- CI 继续 stable + 本地 nightly：信号不一致，风险高。
- CI 同时跑 stable/nightly：超出当前范围，且会增加维护成本。

### Decision 3: Use incremental dependency upgrades under nightly gates
依赖升级采用“锁文件优先、约束按需”的渐进策略：
1) 先更新 `Cargo.lock` 到目标 nightly 可兼容版本；
2) 再对必要的 `Cargo.toml` 版本约束做小步上调；
3) 每步都通过 nightly 质量门禁（fmt/clippy/tests/build）。

**Rationale:**
- 降低一次性全量升级的回归面。
- 便于在失败时快速定位到具体升级批次。

**Alternatives considered:**
- 一次性全量上调所有依赖：速度快，但问题定位和回滚成本高。

### Decision 4: Treat upgrade process as documented policy
把“何时升级 nightly、如何升级依赖、如何验证、如何回滚”作为仓库级维护策略写入变更规范与任务，避免仅靠口头约定。

**Rationale:**
- 降低人员变化带来的知识丢失。
- 保证后续升级动作一致、可审核。

## Risks / Trade-offs

- [nightly 回归风险] → 通过按日期 pin + 小步升级 + 全量门禁验证缓解。
- [依赖升级引入行为差异] → 先锁文件、后约束，分批提交并在每批执行测试与 smoke checks。
- [CI 配置调整造成短期失败] → 在同一变更中同步修正注释/文档/命令，确保单次收敛。
- [维护频率上升] → 通过明确升级触发条件（如月度或安全修复驱动）控制节奏。

## Migration Plan

1. 将 CI 工具链安装策略改为 nightly 基线一致路径，并更新不一致注释。
2. 将 `rust-toolchain.toml` 升级到新的固定 nightly 日期。
3. 在该 nightly 下执行依赖升级（锁文件优先，约束按需）。
4. 执行 nightly 质量门禁与手动 smoke checks；记录结果。
5. 合并后以同一流程执行后续周期性升级。

**Rollback strategy**
- 若升级后出现阻塞问题：回退 `rust-toolchain.toml` 与 `Cargo.lock` 到上一已知稳定提交；必要时回退个别依赖约束变更。

## Open Questions

- nightly 升级节奏采用“固定周期（月度）”还是“事件驱动（仅依赖/安全需求触发）”？
- 依赖约束是否统一改为更宽松写法，还是继续显式固定当前最小版本以保持可控性？
