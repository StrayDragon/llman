# dependency-upgrade-workflow Specification

## Purpose
TBD - created by archiving change deps-and-toolchain-upgrade. Update Purpose after archive.
## Requirements
### Requirement: Dependency upgrades use a lockfile-first sequence
Maintainers MUST perform dependency upgrades in a lockfile-first sequence under the pinned nightly baseline, updating `Cargo.toml` constraints only when lockfile-only updates are insufficient.

#### Scenario: Upgrade run starts
- **WHEN** maintainers start a dependency upgrade for this repository
- **THEN** they first update `Cargo.lock` and run validation before changing dependency version constraints in `Cargo.toml`

### Requirement: Constraint changes are minimal and justified
Any dependency version-constraint changes in `Cargo.toml` MUST be minimal, scoped to compatibility needs, and validated with project quality gates.

#### Scenario: Lockfile-only update fails due to version bounds
- **WHEN** an upgrade requires manifest constraint changes to compile or pass checks
- **THEN** maintainers apply only the required bound updates and verify the result with nightly-based checks

### Requirement: Upgrade outcomes are verifiable and auditable
Dependency upgrade work MUST produce verifiable outcomes, including the final validation result and a fallback path to the previous known-good lock state.

#### Scenario: Upgrade is prepared for merge
- **WHEN** maintainers complete a dependency upgrade batch
- **THEN** they can show that nightly-based checks passed and that reverting to the previous lock state remains possible
