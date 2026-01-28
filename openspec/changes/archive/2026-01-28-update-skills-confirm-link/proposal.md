## Why
- `llman skills` currently re-links ("steals") source skill directories without explicit consent, which can surprise users in interactive sessions.
- We need an explicit, safe opt-in path for source relinking while preserving automation options.

## What Changes
- Add an explicit switch to allow source relinking (flag: `--relink-sources`) in both interactive and non-interactive modes.
- When interactive and relinking is allowed, prompt for a second confirmation; default choice is "no" and exits without changes.
- Add `--yes` to skip the confirmation prompt when interactive.
- Update messages/help and the skills-management spec accordingly.

## Impact
- Specs: `skills-management` requirement updates.
- Code: `src/skills/command.rs`, prompt helpers, i18n strings, and tests for CLI behavior.
- User-visible behavior: interactive `llman skills` will no longer modify sources unless explicitly opted in.
- Rollback: remove the new flag/prompt gate to restore current behavior.
