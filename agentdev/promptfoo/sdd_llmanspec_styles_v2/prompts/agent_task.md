You are running inside an isolated temp git repository (the current working directory).

Rules:
- Only modify files inside this repository.
- The workspace has been pre-seeded with a valid baseline spec + change.
- You MUST use the `Read` tool to inspect `spec.md` files before editing them.
- For required changes, edit the `spec.md` files directly (file-level edits), not via `llman sdd spec add-*` / `llman sdd delta add-*`.
- After major milestones, create a `git commit` (the runner will snapshot via `git log/diff/status`).
- Always finish by running `llman sdd validate --all --strict --no-interactive`.

Task:
{{ task_prompt }}
