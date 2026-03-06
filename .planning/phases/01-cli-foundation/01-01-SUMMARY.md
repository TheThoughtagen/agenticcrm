---
phase: 01-cli-foundation
plan: 01
subsystem: cli
tags: [rust, frontmatter, yaml, validation, json-output, clap]

# Dependency graph
requires: []
provides:
  - "Raw-text frontmatter editor preserving YAML comments and field order"
  - "Contact validation (required fields, enum values, cadence)"
  - "OutputFormat enum with human/JSON output for all commands"
  - "ContactFile.raw_frontmatter field for round-trip safe editing"
  - "Global --format flag on CLI"
affects: [01-02, 01-03]

# Tech tracking
tech-stack:
  added: [serde_json, regex]
  patterns: [raw-frontmatter-preservation, serialize-display-pattern, validation-before-write]

key-files:
  created:
    - src/frontmatter.rs
    - src/validation.rs
    - src/format.rs
  modified:
    - src/models/contact.rs
    - src/store.rs
    - src/main.rs
    - src/commands/list.rs
    - src/commands/show.rs
    - src/commands/search.rs
    - src/commands/due.rs
    - src/commands/log.rs
    - src/commands/add.rs
    - Cargo.toml

key-decisions:
  - "Serialize+Display pattern for all command output types enabling human/JSON dual output"
  - "Raw frontmatter stored as String field on ContactFile, used for serialization instead of serde_yaml::to_string"
  - "Validation runs on every write_contact call, not just specific commands"
  - "New contacts generated from template file via update_field calls to preserve comments"

patterns-established:
  - "Serialize+Display: Every command output type implements both Serialize and Display, enabling format::output to switch on OutputFormat"
  - "Raw frontmatter preservation: ContactFile carries raw_frontmatter String, serialize_contact_file uses it instead of re-serializing"
  - "Validate-before-write: store::write_contact always calls validate_contact before writing to disk"
  - "output_list: For list-like outputs, use format::output_list which handles count footer in human mode"

requirements-completed: [CLI-01, CLI-03, CLI-04]

# Metrics
duration: 4min
completed: 2026-03-05
---

# Phase 1 Plan 01: Core Infrastructure Summary

**Raw-text frontmatter editor, contact validation, and JSON output formatting with global --format flag wired into all 6 CLI commands**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-06T02:06:25Z
- **Completed:** 2026-03-06T02:10:52Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments
- Frontmatter editor that preserves YAML comments, blank lines, and field order during round-trip editing
- Contact validation checking required fields (id, name) and enum values (follow_up_cadence)
- All 6 commands (list, show, search, due, log, add) support --format json producing valid JSON
- 19 unit tests covering frontmatter editing, validation, and output formatting

## Task Commits

Each task was committed atomically:

1. **Task 1: Create frontmatter editor, validation module, and format module** - `fab440d` (feat)
2. **Task 2: Wire infrastructure into store, model, and CLI** - `976814d` (feat)

## Files Created/Modified
- `src/frontmatter.rs` - Raw-text frontmatter parser and editor (parse_raw_frontmatter, update_field, update_array_field)
- `src/validation.rs` - Contact validation with ValidationError type
- `src/format.rs` - OutputFormat enum (Human/Json) with output/output_list helpers
- `src/models/contact.rs` - Added raw_frontmatter field to ContactFile
- `src/store.rs` - Uses raw frontmatter for serialization, validates before write, generate_raw_frontmatter from template
- `src/main.rs` - Global --format flag, passes OutputFormat to all commands
- `src/commands/*.rs` - All 6 commands updated with Serialize+Display output types
- `Cargo.toml` - Added serde_json and regex dependencies

## Decisions Made
- Used Serialize+Display pattern for command outputs rather than separate human/JSON formatting logic -- enables clean format::output dispatch
- Raw frontmatter stored as plain String on ContactFile -- simple and sufficient for comment preservation
- Validation enforced at write_contact level (not per-command) -- ensures no bad data written regardless of code path
- New contacts use template file + update_field to generate frontmatter -- preserves template comments in new contacts

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Frontmatter editor ready for edit/delete/archive commands (plan 02)
- Validation ready to be extended with additional checks
- Format module ready for new commands to use Serialize+Display pattern
- One compiler warning: update_array_field unused (will be used by edit command in plan 02)

---
*Phase: 01-cli-foundation*
*Completed: 2026-03-05*
