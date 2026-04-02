You are running inside an isolated temp git repository (the current working directory).

Rules:
- Only modify files inside this repository.
- Use deterministic CLI commands where possible.
- After major milestones, create a `git commit` (the runner will snapshot via `git log/diff/status`).
- Always finish by running `llman sdd validate --all --strict --no-interactive`.

Task:
{{ task_prompt }}
