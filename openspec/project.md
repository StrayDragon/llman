# Project Spec: llman CLI quality uplift

## Context
- Primary usage: interactive CLI
- Distribution: cargo install
- Platforms: Linux and macOS (Windows is partial and not a goal)
- Compatibility: output and exit codes do not need to be preserved

## Constraints
- Tests and dev commands should not touch real user config; use LLMAN_CONFIG_DIR when running
- Keep changes incremental and reviewable (one task per PR/merge)

## Goals
- Improve maintainability and reduce duplicated logic
- Improve readability and separation of concerns
- Improve reliability and error signaling
- Improve testability and CI signal quality
- Improve CLI experience: error messages, help, and consistency

## Non-goals
- No large new frameworks or rewrites
- No full Windows support expansion
- No breaking changes without a documented rollback path

## Guiding principles
- Small, mergeable steps (one task at a time)
- Prefer reuse of shared helpers in src/config.rs and src/path_utils.rs
- Fail loudly and consistently for errors
- Avoid risky behavior when parsing or modifying user files

## Risks and mitigations
- R1: Output and exit code changes can surprise users
  - Mitigation: document changes per task and provide examples
- R2: Config path changes can move data location
  - Mitigation: keep the same default path; add explicit migration notes
- R3: Safer comment cleaning may remove fewer comments
  - Mitigation: warn clearly and allow opt-in for risky fallback in future
- R4: CI becomes stricter and may slow feedback
  - Mitigation: keep steps minimal; prefer just check over new jobs

## Milestones
- M1: Consistent configuration path resolution and error/exit handling
- M2: Cursor export correctness and safer tool behavior
- M3: Quality gates (fmt/clippy) and message consistency

## Compatibility strategy
- Output changes are acceptable but must be documented per task
- Exit code policy is centralized and applied consistently
- Clap usage/help behavior remains default unless documented

## Acceptance overview
- cargo +nightly fmt -- --check passes
- cargo +nightly clippy --all-targets --all-features -- -D warnings passes
- cargo +nightly test --all passes
- Manual smoke checks for:
  - llman x cc
  - llman x codex
  - llman x cursor
  - llman prompt
  - llman tool
- Document any user-visible output or exit code changes per task
