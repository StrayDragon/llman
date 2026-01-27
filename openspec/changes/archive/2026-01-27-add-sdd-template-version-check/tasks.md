## 1. Design
- [x] 1.1 Confirm metadata format (frontmatter `llman-template-version: 1` or `<!-- llman-template-version: 1 -->`) and scope (all SDD locale templates).

## 2. Implementation
- [x] 2.1 Remove `templates/sdd/spec-driven/`.
- [x] 2.2 Add version metadata to `templates/sdd/<locale>/**/*.md`.
- [x] 2.3 Add template version check script and `just check-sdd-templates` command.

## 3. Validation
- [x] 3.1 Run `just check-sdd-templates`.
