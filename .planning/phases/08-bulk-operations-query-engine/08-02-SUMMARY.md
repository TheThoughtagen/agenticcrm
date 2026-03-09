---
phase: 08-bulk-operations-query-engine
plan: 02
subsystem: cli
tags: [bulk-cli, preview-confirm, dry-run, stdin-pipe, unix-composability]

# Dependency graph
requires:
  - phase: 08-bulk-operations-query-engine
    provides: "Query engine, bulk ops functions (query, bulk_update, bulk_delete, bulk_archive, bulk_tag)"
provides:
  - "acrm bulk subcommand with query-based matching and bulk actions"
  - "acrm bulk-update subcommand with JSON stdin pipe support"
  - "Preview/confirm UX with --yes skip and --dry-run mode"
  - "--delete and --archive mutual exclusivity via clap"
affects: [09-mcp-server]

# Tech tracking
tech-stack:
  added: []
  patterns: [preview-confirm-ux, stdin-json-pipe, dry-run-cli-pattern]

key-files:
  created:
    - src/commands/bulk.rs
  modified:
    - src/commands/mod.rs
    - src/main.rs

key-decisions:
  - "Shared helper functions between run_bulk and run_bulk_update to avoid duplication"
  - "Preview truncates at 20 contacts with '...and N more' for readability"
  - "TTY detection on stdin to provide helpful error when --stdin used without pipe"
  - "Query-only mode (no action flags) displays matched contacts as a list"

patterns-established:
  - "Bulk CLI preview pattern: list affected contacts, show action, confirm before executing"
  - "Stdin JSON pipe pattern: search --format json | bulk-update --stdin for Unix composability"
  - "Dry-run CLI pattern: [DRY RUN] prefix with full change list, no writes"

requirements-completed: [BULK-05, BULK-06, BULK-07]

# Metrics
duration: 5min
completed: 2026-03-09
---

# Phase 8 Plan 2: Bulk CLI Wiring Summary

**CLI commands for bulk operations with preview/confirm UX, dry-run support, and Unix stdin pipe composability**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-09T14:20:00Z
- **Completed:** 2026-03-09T14:25:20Z
- **Tasks:** 2 (1 auto + 1 human-verify checkpoint)
- **Files modified:** 3

## Accomplishments
- `acrm bulk` command with query predicates and all action flags (--set, --delete, --archive, --add-tag, --remove-tag)
- `acrm bulk-update` command reading JSON contact list from stdin for Unix pipe composability
- Preview/confirm UX: shows affected contacts and planned action, prompts before executing
- --dry-run shows full change preview without writing, --yes skips confirmation prompt
- --delete and --archive mutually exclusive via clap conflicts_with

## Task Commits

Each task was committed atomically:

1. **Task 1: Create bulk CLI commands with preview/confirm/dry-run and stdin pipe** - `bf6c870` (feat)
2. **Task 2: Verify bulk operations end-to-end** - checkpoint approved by user

## Files Created/Modified
- `src/commands/bulk.rs` - 362 lines: run_bulk() and run_bulk_update() handlers with preview, confirm, dry-run, and JSON/human output
- `src/commands/mod.rs` - Added `pub mod bulk` declaration
- `src/main.rs` - Added Bulk and BulkUpdate variants to Commands enum with clap arg definitions and dispatch

## Decisions Made
- Shared helper functions between run_bulk and run_bulk_update to keep DRY
- Preview truncates contact list at 20 entries with "...and N more" for terminal readability
- TTY detection on stdin provides helpful usage error when --stdin flag used without actual pipe input
- Query-only mode (no action flags) displays matched contacts as a table, supporting both human and JSON output formats

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None - implementation matched plan specification.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All bulk operations fully wired to CLI: query, update, delete, archive, tag
- Phase 08 complete: query engine + bulk ops + CLI wiring all functional
- Ready for Phase 09 (MCP Server) which can reuse ops layer functions directly

---
*Phase: 08-bulk-operations-query-engine*
*Completed: 2026-03-09*
