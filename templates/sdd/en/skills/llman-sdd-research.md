---
name: "llman-sdd-research"
description: "Delegate external research to a background agent. Use when the user needs official docs/API/source facts gathered, or wants the reading legwork delegated so they can keep working."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Research

Spin up a **background agent** to do the research, so you keep working while it reads.

## Pipeline position

Auxiliary tool, usable at any stage. Common in explore/wayfinder to provide factual input for decisions. Output is written back to the change's proposal "Further Notes" section for later stages to consume.

> 📍 Standalone optional skill; research output feeds the main flow's explore/propose.

## Responsibilities

The background agent's job:

1. Investigate the question against **primary sources** — official docs, source code, specs, first-party APIs — not secondary write-ups. Follow every claim back to the source that owns it.
2. Write findings to a single Markdown file, citing each claim's source.
3. Save where the repo already keeps such notes; match the existing convention, and if there is none, put it at `llmanspec/changes/<current-change>/research/<topic>.md` and say where.

## Steps

1. Clarify the research question (confirm with the user; if fuzzy, sharpen to a falsifiable one).
2. Use the Agent tool `subagent_type=general-purpose` + `run_in_background: true` to launch the background research, with a prompt containing:
   - The question statement.
   - A requirement to cite only primary sources, with source URL/path per claim.
   - The output file path (`llmanspec/changes/<id>/research/<topic>.md`).
   - A word limit (suggested: focus on facts, prose narrative < 1500 words).
3. Continue main-flow work while it runs in the background; receive a notification when done.
4. Read the output, summarize key conclusions back into the current change's `proposal.md` "Further Notes" section (with a file pointer).
5. If the research reveals a decision is needed, suggest entering `llman-sdd-explore`'s grilling branch.

## Cooperation with wayfinder

`llman-sdd-wayfinder`'s research tickets delegate to this skill for background resolution; on completion, write back to the ticket proposal and record a one-line gist in the map's Decisions-so-far.

{{ unit("skills/structured-protocol") }}
