## ADDED Requirements

### Requirement: Structured Protocol Includes Ethics Governance Fields
The structured skill prompt protocol for new style MUST include enforceable ethics governance fields.

#### Scenario: New style structured protocol includes governance block
- **WHEN** new style SDD skills are generated
- **THEN** generated content includes governance fields for risk level, prohibited actions, required evidence, refusal contract, and escalation policy

### Requirement: Missing Governance Fields Fail New-Style Validation
New-style validation MUST fail when required ethics governance fields are missing.

#### Scenario: Validation fails on missing governance key
- **WHEN** a new style skill/protocol artifact omits a required ethics governance field
- **THEN** validation returns non-zero with explicit missing-field diagnostics
