# tool-clean-comments Specification

## Purpose
Define clean-comments safety behavior and the tree-sitter-only removal path.
## Requirements
### Requirement: Safe failure on tree-sitter unavailability
When tree-sitter is unavailable or fails for a file, the clean-comments processor MUST skip modification for that file and record an error while continuing other files.

#### Scenario: Tree-sitter unavailable
- **WHEN** tree-sitter cannot be initialized
- **THEN** no files are modified and errors are reported

#### Scenario: Tree-sitter fails on a file
- **WHEN** tree-sitter fails while processing a specific file
- **THEN** that file remains unchanged and processing continues for remaining files

### Requirement: Regex fallback is disabled by default
Regex-based comment removal MUST NOT run by default; it may remain available only for explicit future opt-in.

#### Scenario: Default run
- **WHEN** clean-comments runs without any explicit opt-in
- **THEN** regex-based removal is not used
