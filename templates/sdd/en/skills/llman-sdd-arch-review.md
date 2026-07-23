---
name: "llman-sdd-arch-review"
description: "Scan codebase for shallow modules (interface nearly equals implementation) and surface deepening candidates. Use when the user wants an architecture review, seeks module deepening opportunities, or wants to improve testability and AI-navigability."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Architecture Review

Scan the codebase for architectural friction and surface **deepening opportunities** — refactors that turn shallow modules (interface nearly equals implementation) into deep ones (lots of behaviour behind a small interface). The aim is testability and AI-navigability.

## Pipeline position

Auxiliary tool, not part of the main pipeline (explore→propose→apply→verify→archive). Usable at any stage; commonly triggered during explore to surface improvement candidates.

> 📍 Standalone optional skill; does not replace any pipeline stage.

## Design vocabulary

A set of words about module shape, used to articulate "where it's worth changing". MUST NOT substitute "component" / "service" / "API" / "boundary" (they are broader, less precise):

- **Module** — anything with an interface and an implementation (function/class/package/cross-layer slice).
- **Interface** — everything a caller must know to use it correctly: type signature, plus invariants, ordering constraints, error modes, performance characteristics.
- **Depth** — the amount of behaviour behind the interface. **Deep** = lots of behaviour behind a small interface; **shallow** = interface nearly as complex as the implementation (the caller saves nothing).
- **Seam** — a place where you can swap the implementation without editing call sites (where the interface lives). In llman, seam = the public boundary driven by `*.feature` GWT steps.
- **Leverage** — what callers get from depth: more capability per unit of interface learned.
- **Locality** — what maintainers get from depth: changes/bugs/knowledge/verification concentrate in one place.

## Steps

### 1. Explore (scope first, YAGNI)
- If the user named a direction (module/subsystem/pain point), accept it; skip inference.
- Otherwise walk `git log --oneline` for hot spots (files/areas that keep coming up).
- Prefer reading live `spec.toon` (BDD-on, domain SSOT) and `design.md` (existing ADRs); MUST NOT create a `CONTEXT.md`.
- Use the Agent tool (`subagent_type=Explore`) to walk the codebase, noting friction:
  - Does understanding one concept require bouncing between many small modules?
  - Where are modules **shallow** (interface nearly as complex as the implementation)?
  - Where are pure functions extracted only for testability, but real bugs hide in how they're called (no locality)?
  - Which parts are untested or hard to test through their current interface?

### 2. Present candidates
For each candidate:
- **Files** — which files/modules are involved.
- **Problem** — why the current architecture causes friction (use depth/leverage/locality).
- **Solution** — plain-English description of what would change.
- **Benefits** — locality and leverage improvements; how tests get better.
- **Recommendation strength** — `Strong` / `Worth exploring` / `Speculative`.

**Deletion test**: for any suspected-shallow module, imagine deleting it — does complexity vanish (it's just a pass-through, no value) or reappear across N call sites (it's actually earning its keep)? "Reappears" is the signal you want.

**ADR conflicts**: if a candidate contradicts an existing `design.md` decision, surface it only when the friction is real enough to warrant reopening, and mark it in the candidate.

### 3. Grilling (after the user picks a candidate)
Run `llman-sdd-explore`'s **grilling branch** (trigger "deep-dig") to walk the decision tree — constraints, dependencies, the deepened module's shape, what sits behind the seam, which tests survive.

- A deepened module uses a concept not in `spec.toon`? → update the `spec.toon` requirement statement (r107, BDD-on: edit live file on the feature branch).
- User rejects the candidate with a load-bearing reason? → offer an ADR only when "hard to reverse + surprising without context + real trade-off" all hold; record in `design.md`.

## Output
Candidate list (text; optional HTML report written to OS temp dir, not the repo) + the grilling decision record after the user picks one (written back to proposal/spec.toon).

{{ unit("skills/structured-protocol") }}
