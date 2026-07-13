# Design: feat-sdd-status-compact

## Status Command Interface

```
llman sdd status [OPTIONS] [TARGET]

OPTIONS:
  --format toon|json    Output format (default: toon)
  --json                Shorthand for --format json (backward compat)
```

## Output Format: TOON

### Project-level (no TARGET)

```toon
kind: llman.sdd.status
counts{active,specs}:
  2,34,
changes[2]{name,stage,tasks,next}:
  c1,feat-spec-agent-interface,full,3/5,"t4: validate --strict",
  c2,feat-spec-quality-triage,draft,0/0,propose,
```

- `changes` table: only active changes, sorted by priority
- `next` column: first incomplete task's test command, or `propose`/`archive`
- No mermaid, no markdown prose

### Single-change (TARGET = change name)

```toon
kind: llman.sdd.status
change{name,stage,priority,tasks}:
  feat-spec-agent-interface,full,c1,3/5,
tasks[2]{id,title,test}:
  t4,"Test edge cases","validate --strict",
  t5,"Update docs",
next: "impl t4 + validate --strict + just check"
```

- `tasks` table: only **incomplete** tasks (never completed ones)
- `test` column: test command from tasks.md if available
- `next`: concrete next action line

### Archived (TARGET = date prefix or fuzzy)

```toon
kind: llman.sdd.status
change{name,status,archived,commit}:
  2026-07-13-harden-git-ref-and-env-injection,archived,2026-07-14,abc1234,
ops[2]{op,req_id,title}:
  add_req,r1,"Git refs sanitized",
  add_req,r2,"Env injection prevented",
```

## JSON Output (--format json / --json)

Keeps existing format unchanged:
```json
{
  "activeChanges": 2,
  "draft": 0,
  "specified": 1,
  "designed": 0,
  "full": 1,
  "pendingValidation": 0,
  "specs": 34
}
```

For single-change TARGET with --json, add task detail:
```json
{
  "change": "feat-spec-agent-interface",
  "stage": "full",
  "completedTasks": 3,
  "totalTasks": 5,
  "nextAction": "impl t4"
}
```

## Target Resolution Order

1. Exact match `llmanspec/changes/<name>/`
2. Date-prefix match: `llmanspec/changes/archive/YYYY-MM-DD-*`
3. Substring fuzzy match across all (active + archive) names
4. No match → error with nearest suggestions

## Priority Detection

1. `c<N>-` prefix in directory name → N is priority
2. No prefix → alphabetical fallback
3. Lower N = higher priority (c1 before c2)

## Apply-Cycle Skill

- `disable-model-invocation: true` → hidden from agent's `available_skills`
- User triggers via `/skill:llman-sdd-apply-cycle <change-id>`
- First action: `llman sdd status <change-id>` → parse TOON output
- Loop: for each incomplete task in `tasks[]`, implement → test → next
- After all tasks: `llman sdd validate --strict` → `archive run` → git commit
- Hard constraints: no asking, no switching, retry 3x before blocker
