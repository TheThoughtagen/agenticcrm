---
phase: 07-operations-layer
plan: 01
subsystem: api
tags: [ops-layer, thiserror, refactoring, multi-consumer-architecture]

# Dependency graph
requires:
  - phase: 01-cli-foundation
    provides: "CLI command handlers, store.rs, frontmatter.rs, models, validation"
provides:
  - "src/ops/ module with OpsError enum and typed result structs"
  - "ops::contact CRUD functions (add, list, search, show, edit, log_interaction, due, find_delete_target, confirm_delete, archive, unarchive)"
  - "Two-phase delete pattern for non-interactive consumers"
  - "Thin CLI wrappers delegating to ops layer"
affects: [08-bulk-operations, 09-mcp-server, 07-02-sync-ops-extraction]

# Tech tracking
tech-stack:
  added: [thiserror]
  patterns: [ops-layer-extraction, thin-cli-wrapper, typed-error-enum, two-phase-delete]

key-files:
  created:
    - src/ops/mod.rs
    - src/ops/error.rs
    - src/ops/contact.rs
  modified:
    - src/commands/add.rs
    - src/commands/list.rs
    - src/commands/search.rs
    - src/commands/show.rs
    - src/commands/edit.rs
    - src/commands/log.rs
    - src/commands/due.rs
    - src/commands/delete.rs
    - src/commands/archive.rs
    - src/main.rs
    - Cargo.toml

key-decisions:
  - "OpsError uses thiserror with NotFound, AmbiguousMatch, ValidationFailed, Io, Internal variants"
  - "ops::contact owns fuzzy name matching via internal find_contact helper"
  - "Display impls with colored output stay in command files, ops structs get Serialize+Debug only"
  - "Two-phase delete: find_delete_target returns info, confirm_delete performs deletion"
  - "next_follow_up and needs_quoting moved to ops as shared business logic utilities"

patterns-established:
  - "Ops function signature: fn(root: &Path, args...) -> Result<TypedResult, OpsError>"
  - "CLI thin wrapper: find_crm_root() + ops call + format::output()"
  - "Error mapping: anyhow results map to OpsError::Internal via .map_err(internal)"
  - "Two-phase delete: find target then confirm, enabling non-interactive consumers"

requirements-completed: [OPS-01, OPS-02]

# Metrics
duration: 8min
completed: 2026-03-09
---

# Phase 7 Plan 1: CRUD Ops Extraction Summary

**Extracted all 9 CRUD command business logic into shared ops/contact.rs with OpsError enum, typed results, and thin CLI wrappers**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-09T13:00:51Z
- **Completed:** 2026-03-09T13:09:07Z
- **Tasks:** 2
- **Files modified:** 15

## Accomplishments
- Created ops module with OpsError enum (5 variants) using thiserror for typed error matching
- Extracted all 9 CRUD commands (add, list, search, show, edit, log, due, delete, archive) into ops functions
- Implemented two-phase delete pattern (find_delete_target + confirm_delete) for MCP/bulk consumers
- All 131 tests pass with zero behavioral regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create ops module with OpsError and all CRUD result types** - `5f5b3c5` (feat)
2. **Task 2: Extract CRUD business logic into ops functions, thin-wrap CLI handlers** - `e1f8cd8` (feat)

## Files Created/Modified
- `src/ops/mod.rs` - Module root with re-exports
- `src/ops/error.rs` - OpsError enum with thiserror derives
- `src/ops/contact.rs` - All CRUD business logic functions and result structs
- `src/commands/add.rs` - Thin wrapper calling ops::contact::add
- `src/commands/list.rs` - Thin wrapper calling ops::contact::list
- `src/commands/search.rs` - Thin wrapper calling ops::contact::search
- `src/commands/show.rs` - Thin wrapper calling ops::contact::show
- `src/commands/edit.rs` - Thin wrapper calling ops::contact::edit
- `src/commands/log.rs` - Thin wrapper calling ops::contact::log_interaction
- `src/commands/due.rs` - Thin wrapper calling ops::contact::due
- `src/commands/delete.rs` - Thin wrapper with two-phase delete pattern
- `src/commands/archive.rs` - Thin wrapper calling ops::contact::archive/unarchive
- `src/main.rs` - Added `mod ops` declaration
- `Cargo.toml` - Added thiserror dependency
- `Cargo.lock` - Updated lockfile

## Decisions Made
- Used thiserror 2.x for OpsError enum (standard Rust approach, enables downstream matching)
- Created internal `find_contact` helper in ops that returns OpsError::NotFound/AmbiguousMatch (store::find_single_contact returns anyhow, ops needs typed errors)
- Kept backward-compatible `next_follow_up` re-export in commands/log.rs for TUI compatibility
- Kept empty-result handling (printing "No contacts found." etc.) in CLI wrappers since it's presentation logic

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- ops/contact.rs ready for Phase 8 (bulk operations) and Phase 9 (MCP server) to consume
- Sync ops extraction (Plan 07-02) is next -- same pattern for sync commands
- store::find_single_contact now has dead_code warning since ops uses its own find_contact -- can be cleaned up in 07-02

---
*Phase: 07-operations-layer*
*Completed: 2026-03-09*
