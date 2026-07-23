---
name: "llman-sdd-wayfinder"
description: "Plan a huge, foggy chunk of work (more than one agent session can hold) as a shared map of decision tickets, resolving them one at a time until the way is clear. Manual trigger only; the agent must not auto-invoke."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Wayfinder

A loose, large idea has arrived — too big for a single agent session, wrapped in fog: the way from here to the **destination** isn't visible yet. This skill finds that way rather than charging at the destination.

It charts the path as an llman SDD **change dependency graph** (`llman sdd graph`): each sub-work (ticket) resolves a **decision** rather than delivering a slice, worked one at a time until the way is clear.

## Pipeline position

Auxiliary tool, for **pre-planning large work** before the main pipeline. When the map clears, merge onto the main flow at `llman-sdd-propose`.

> 📍 Standalone optional skill; when the map clears → `llman-sdd-propose` (collapse decisions into a buildable plan).

## Core principles

- **Plan, don't do**: each ticket resolves a decision; the map is done when "the way is clear, no decisions left". The urge to just do the work is usually the signal you've reached the map's edge and should hand off.
- **Refer by name**: in all human-readable narration, refer to a ticket by its title; MUST NOT use bare ids/numbers.
- **One session, one ticket**: each session resolves only one ticket (research tickets excepted).

## Map structure

The map is a change (overview proposal); its sub-decisions are `depends_on` child changes. Use `llman sdd graph <map-id> --scope active` to visualize the **frontier** (takeable items).

The map's `proposal.md` structure:

```markdown
## Destination
<what reaching the end looks like — spec/decision/change. One or two lines.>

## Notes
<domain; skills each session should consult; standing preferences for this effort>

## Decisions so far
<!-- index: one line per closed ticket, gist + link -->

## Not yet specified
<!-- fog: foreseeable but not yet sharp enough to ticket; graduates as the frontier advances -->

## Out of scope
<!-- beyond the destination; closed tickets, never graduate -->
```

## Ticket types

Each ticket is a child change carrying a `wayfinder:<type>` tag (in the proposal title or frontmatter):

- **Research (agent-driven)**: read docs/APIs/local resources to surface a fact a decision waits on. Delegate to `llman-sdd-research` in the background.
- **Prototype (human-in-the-loop)**: raise fidelity with a cheap, rough runnable (throwaway terminal app or UI variant).
- **Grilling (human-in-the-loop)**: via `llman-sdd-explore`'s grilling branch, one question at a time. **Default type**.
- **Task (human or agent)**: manual work that must happen before a decision can be made (sign up for a service, move data so its shape is visible).

## Fog of war

The map is **deliberately** incomplete. The test for ticket-vs-fog: **can you state the question precisely now** (not whether you can answer it).
- Can state precisely → ticket (even if blocked).
- Cannot yet state precisely → **Not yet specified** (coarser than a ticket; one fog patch may graduate into several tickets or none).

## Steps

### Chart the map
1. **Name the destination**: use `llman-sdd-explore`'s grilling branch to pin down what this map is finding its way to.
2. **Breadth-first scan**: grill again, fanning out rather than deep-diving, surfacing open decisions and the first takeable steps. If **no fog surfaces** — the way is already clear, the whole effort fits one session — you don't need a map; stop and ask the user how to proceed.
3. **Create the map** (overview change): `llman sdd change new <map-id>`, fill Destination/Notes, leave Decisions-so-far empty, write fog into Not yet specified.
4. **Create the tickets you can specify now** as child changes, then wire blocking edges with `llman sdd graph` (second pass: ids needed before cross-referencing).
5. Spin up `llman-sdd-research` background subagents for each research ticket.
6. Stop — charting is one session's work; resolve nothing by hand.

### Work through the map
1. Load the map (low-resolution view).
2. Pick a ticket (user-named or first frontier item), `change attach` to claim it.
3. Resolve it — zoom as needed (read related ticket bodies, invoke skills the Notes block names). In doubt, use `llman-sdd-explore`'s grilling.
4. Record the resolution: write the answer into the ticket's proposal, close it, append a one-line gist + pointer to the map's Decisions-so-far.
5. Add newly-surfaced tickets (create-then-wire); graduate fog that the answer has made specifiable, clearing it from Not yet specified. If the answer reveals a ticket sits beyond the destination, rule it out of scope rather than resolving it on the route.

## Output
Map change + child decision changes' dependency graph (`llman sdd graph`). When the way is clear, proceed to `llman-sdd-propose` to collapse the decisions into a buildable plan.

{{ unit("skills/structured-protocol") }}
