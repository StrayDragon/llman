## ADDED Requirements

### Requirement: Built-In Old-vs-New Evaluation Flow
The SDD workflow MUST provide an evaluation flow that compares legacy and new style outputs on the same scenario set.

#### Scenario: Run evaluation over shared scenarios
- **WHEN** a user executes the evaluation flow for a target scenario set
- **THEN** the system runs both legacy and new style generation/evaluation on equivalent inputs
- **AND** records paired results for comparison

### Requirement: Safety-First Scoring Output
Evaluation outputs MUST prioritize quality and safety metrics over cost metrics.

#### Scenario: Report includes prioritized metrics
- **WHEN** the evaluation report is generated
- **THEN** it includes quality and safety scores before token/latency metrics
- **AND** it marks pass/fail gates for safety-sensitive checks
