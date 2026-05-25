---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - llman sdd validate sdd-legacy-compat --type spec --strict --no-interactive
llman_spec_evidence:
  - migrated from openspec
---

```toon
kind: llman.sdd.spec
name: "sdd-legacy-compat"
purpose: "TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive."
requirements[1]{req_id,title,statement}:
  r1,Legacy Track Is Retired,"The SDD workflow MUST NOT provide a legacy track (`sdd-legacy`) for templates/skills/prompts."
scenarios[1]{req_id,id,given,when,then}:
  r1,"user-tries-to-use-legacy-track","","a user looks for or attempts to use legacy-style SDD commands or templates",the system fails loudly
```
