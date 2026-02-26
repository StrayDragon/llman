## ADDED Requirements

### Requirement: Compaction Guidance Must Reference ISON Source of Truth
Compaction guidance in new style MUST define ISON source artifacts as canonical for compaction decisions.

#### Scenario: Compaction flow references canonical ISON source
- **WHEN** a user follows specs compaction guidance in new style
- **THEN** keep/merge/remove decisions are derived from ISON source artifacts
- **AND** rendered Markdown is treated as a compatibility surface

### Requirement: Compaction Guidance Includes Safety Regression Check
Compaction guidance MUST include a safety regression comparison step between baseline and compacted outputs.

#### Scenario: Compaction includes safety check gate
- **WHEN** a compaction plan is prepared
- **THEN** the workflow includes a before/after safety check gate
- **AND** warns users to stop when safety-critical behavior changes
