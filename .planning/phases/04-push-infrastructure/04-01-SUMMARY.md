---
phase: 04-push-infrastructure
plan: 01
subsystem: sync
tags: [vcard, calcard, serialization, cache, carddav]

# Dependency graph
requires:
  - phase: 02-carddav-sync
    provides: "vcard_map.rs vCard-to-Contact mapping, calcard dependency"
provides:
  - "contact_to_vcard: Contact -> vCard 3.0 serialization"
  - "merge_contact_to_vcard: lossless round-trip merge preserving iCloud-only properties"
  - "vCard cache management (read/write/delete) in .sync/vcards/"
affects: [04-push-infrastructure, 05-change-detection]

# Tech tracking
tech-stack:
  added: [tempfile (dev)]
  patterns: [parse-overlay-serialize for vCard round-tripping, VCardValue::Text for N property semicolon-separated components]

key-files:
  created: [src/sync/vcard_write.rs]
  modified: [src/sync/mod.rs, .gitignore]

key-decisions:
  - "Use VCardValue::Text (not Component) for N property parts to get semicolon separators from calcard writer"
  - "calcard already outputs CRLF; ensure_crlf is a safety net for edge cases"

patterns-established:
  - "Parse-overlay-serialize: parse cached vCard, remove CRM_MAPPED_PROPERTIES, re-add from Contact, serialize"
  - "Cache path convention: .sync/vcards/{source_id}.vcf"

requirements-completed: [PUSH-04]

# Metrics
duration: 3min
completed: 2026-03-07
---

# Phase 4 Plan 1: vCard Serialization and Cache Summary

**Contact-to-vCard 3.0 serialization with parse-overlay-serialize merge pattern preserving iCloud-only properties (X-ABUID, PHOTO, etc.)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-07T15:45:15Z
- **Completed:** 2026-03-07T15:49:01Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 3

## Accomplishments
- contact_to_vcard builds valid vCard 3.0 with VERSION, FN, N, EMAIL, TEL, ORG, TITLE, URL, BDAY, UID
- merge_contact_to_vcard preserves non-CRM properties (X-ABUID, X-ABLABEL, PHOTO) through parse-overlay-serialize
- Cache management functions for .sync/vcards/ directory (read, write, delete)
- All output guaranteed CRLF line endings per vCard 3.0 spec

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): Failing tests for vCard serialization** - `c7ae3f0` (test)
2. **Task 1 (GREEN): Implement vCard serialization and cache** - `6e36e04` (feat)

_TDD task with RED and GREEN commits._

## Files Created/Modified
- `src/sync/vcard_write.rs` - Contact-to-vCard serialization, merge, and cache functions (22 tests)
- `src/sync/mod.rs` - Added vcard_write module registration
- `.gitignore` - Added .sync/ exclusion
- `Cargo.toml` / `Cargo.lock` - Added tempfile dev dependency

## Decisions Made
- Used VCardValue::Text for each N property component (family, given, middle, prefix, suffix) rather than VCardValue::Component, because calcard's writer uses semicolons between values and commas within Component vectors
- calcard's writer already outputs CRLF; ensure_crlf provides safety for edge cases where calcard behavior might change

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] VCardValue::Component produces commas, not semicolons for N property**
- **Found during:** Task 1 (GREEN phase)
- **Issue:** VCardValue::Component separates items with commas, but N property requires semicolon-separated components (Family;Given;;;)
- **Fix:** Used separate VCardValue::Text entries in with_values() instead of single VCardValue::Component
- **Files modified:** src/sync/vcard_write.rs
- **Verification:** test_contact_to_vcard_has_n_structured passes with "N:Smith;Jane;;;"
- **Committed in:** 6e36e04

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Necessary for correct vCard 3.0 N property format. No scope creep.

## Issues Encountered
None beyond the VCardValue::Component vs Text issue documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- vCard serialization module ready for push orchestration (04-02, 04-03)
- Cache functions ready for pull integration (store raw vCards during pull for merge)
- All exports documented in plan: contact_to_vcard, merge_contact_to_vcard, read_cached_vcard, write_cached_vcard, delete_cached_vcard, cache_dir

---
*Phase: 04-push-infrastructure*
*Completed: 2026-03-07*
