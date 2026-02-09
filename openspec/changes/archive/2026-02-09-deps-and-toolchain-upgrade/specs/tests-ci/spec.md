## MODIFIED Requirements

### Requirement: CI runs fmt and clippy with warnings denied
CI MUST run format checks and clippy with `-D warnings` as part of the standard check pipeline, and the checks MUST run on the same pinned nightly baseline used by the repository toolchain configuration.

#### Scenario: CI check job uses pinned nightly baseline
- **WHEN** CI runs on main branch
- **THEN** the check job uses the repository pinned nightly baseline and executes `just check-all` (or an equivalent nightly-based check sequence)

### Requirement: CI runs a release build check
CI MUST run a release build to ensure the project builds on the same pinned nightly baseline used for local development and quality checks.

#### Scenario: CI build job uses pinned nightly baseline
- **WHEN** CI runs on main branch
- **THEN** the build job uses the repository pinned nightly baseline and executes `just build-release`
