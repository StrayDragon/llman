# agentdev/

This directory contains developer-facing assets for **agent workflows** and **prompt evaluation**.

Scope:
- Prompt evaluation fixtures (Promptfoo)
- Runner scripts (local + docker)
- Experiment notes and tooling that are **not** part of the shipping `llman` CLI

Non-goals:
- Do not store user-facing runtime assets here.
- Do not store test config fixtures here (those live under `artifacts/testing_config_home/`).
- Do not commit secrets or credentials.

If you are looking for Promptfoo suites, see `agentdev/promptfoo/`.
