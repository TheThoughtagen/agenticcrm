---
phase: 10-linkedin-import
plan: 01
subsystem: import
tags: [csv, linkedin, serde, dedup, merge]

requires:
  - phase: 07-operations-layer
    provides: "ops module pattern, store API, frontmatter editing"
provides:
  - "import_linkedin() function for CSV-based LinkedIn contact import"
  - "LinkedInRow, ImportResult, ImportChange, ImportSkip, DetectedChange types"
  - "Dedup matching by exact name (case-insensitive) or email"
  - "Fill-empty-only merge with change detection"
affects: [10-02-cli-layer]

tech-stack:
  added: [csv 1.4]
  patterns: [fill-empty-only merge, exact name dedup, array merge with dedup]

key-files:
  created:
    - src/ops/import.rs
  modified:
    - Cargo.toml
    - src/ops/mod.rs

key-decisions:
  - "Date format order: %m/%d/%y before %m/%d/%Y to avoid chrono 2-digit year misparse"
  - "Source field: only fill if truly empty (not if 'manual' default), report detected change for non-linkedin/non-manual sources"
  - "New contacts created via ops::contact::add then post-processed with frontmatter updates"

patterns-established:
  - "Import ops pattern: read CSV -> load existing -> match/dedup -> merge -> write"
  - "Fill-empty-only: check Contact struct field emptiness, use frontmatter::update_field for actual edits"
  - "DetectedChange reporting: track field differences without applying them"

requirements-completed: [LNKD-01, LNKD-02, LNKD-03, LNKD-04]

duration: 4min
completed: 2026-03-09
---

# Phase 10 Plan 01: LinkedIn Import Ops Summary

**LinkedIn CSV import ops with serde deserialization, exact name/email dedup, fill-empty-only merge, change detection, and dry_run support**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-09T18:05:54Z
- **Completed:** 2026-03-09T18:09:45Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- LinkedIn CSV parsing with flexible/trimmed reader and multi-format date parsing
- Dedup matching by exact case-insensitive name OR email with ambiguous match detection
- Fill-empty-only merge: non-empty CRM fields never overwritten, differences reported as DetectedChange
- Array field merge (email, tags) with case-insensitive dedup
- 16 comprehensive tests covering all import scenarios

## Task Commits

Each task was committed atomically:

1. **Task 1: Add csv dependency and create ops/import.rs with types and CSV parsing** - `74bb40a` (feat)
2. **Task 2: Implement tests for import_linkedin with TDD verification** - `db9dbd1` (test)

## Files Created/Modified
- `src/ops/import.rs` - LinkedIn import ops: CSV parsing, dedup matching, merge logic, import_linkedin()
- `src/ops/mod.rs` - Added pub mod import registration
- `Cargo.toml` - Added csv 1.4 dependency

## Decisions Made
- Date format order: try `%m/%d/%y` before `%m/%d/%Y` because chrono parses 2-digit years literally with `%Y`, causing "01/15/24" to become year 0024
- Source field fill logic: only fill if truly empty string, not if set to "manual" (default); report DetectedChange for other non-linkedin values
- New contacts created via existing ops::contact::add() then post-processed with frontmatter field updates for company, role, source, email, met_date, tags, relationship

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed date format ordering for 2-digit year parsing**
- **Found during:** Task 2 (test_parse_connected_on_formats)
- **Issue:** chrono's `%Y` format accepts 2-digit years literally ("24" -> year 0024), so `%m/%d/%Y` matched before `%m/%d/%y` could handle it correctly
- **Fix:** Reordered formats to try `%m/%d/%y` before `%m/%d/%Y`
- **Files modified:** src/ops/import.rs
- **Verification:** test_parse_connected_on_formats passes with all 5 date formats
- **Committed in:** db9dbd1 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix for correct date parsing. No scope creep.

## Issues Encountered
None beyond the date format ordering bug caught by tests.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- import_linkedin() function ready for CLI wrapper in Plan 02
- All types exported from ops::import module
- ImportResult provides summary_counts() for CLI display

---
*Phase: 10-linkedin-import*
*Completed: 2026-03-09*
