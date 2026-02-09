## ADDED Requirements

### Requirement: Pinned nightly is the single build baseline
The repository MUST define a single pinned nightly toolchain baseline in `rust-toolchain.toml`, and maintainers MUST treat it as the authoritative local build toolchain.

#### Scenario: Developer environment resolves toolchain baseline
- **WHEN** a developer runs project build or check commands in this repository
- **THEN** the effective Rust toolchain resolves to the pinned nightly baseline defined by the repository

### Requirement: Nightly bump uses explicit validation gates
When the pinned nightly date is upgraded, maintainers MUST validate the new baseline with the project quality gates before considering the bump complete.

#### Scenario: Maintainer upgrades pinned nightly date
- **WHEN** `rust-toolchain.toml` is changed to a newer nightly date
- **THEN** the change passes nightly-based formatting, lint, tests, and release build checks

### Requirement: Nightly bump must be reversible
Nightly baseline updates MUST keep a documented rollback path to the previous known-good pinned nightly.

#### Scenario: New nightly introduces a blocking regression
- **WHEN** a regression blocks merge or release after a nightly bump
- **THEN** maintainers can restore the prior pinned nightly baseline and recover a green build without rewriting unrelated code
