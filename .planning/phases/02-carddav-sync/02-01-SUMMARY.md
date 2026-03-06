---
phase: 02-carddav-sync
plan: 01
subsystem: sync
tags: [vcard, calcard, carddav, dedup, etag, icloud]

# Dependency graph
requires:
  - phase: 01-cli-foundation
    provides: Contact struct, ContactFile, validation, store module
provides:
  - vCard-to-Contact field mapping (map_vcard_to_contact)
  - Duplicate detection by source_id (find_existing_by_source_id)
  - ETag-based change detection (should_update)
  - Contact struct etag field for sync metadata
affects: [02-carddav-sync]

# Tech tracking
tech-stack:
  added: [calcard 0.3.2]
  patterns: [vCard property extraction with fallback chains, MappedContact return type for contact+notes]

key-files:
  created:
    - src/sync/vcard_map.rs
    - src/sync/dedup.rs
  modified:
    - src/models/contact.rs
    - src/sync/mod.rs
    - .schemas/contact.yaml
    - templates/contact.md
    - src/validation.rs
    - src/sync/carddav.rs

key-decisions:
  - "calcard crate for vCard 3.0/4.0 parsing (Stalwart Labs, production quality)"
  - "MappedContact struct separates contact data from notes for markdown body generation"
  - "Name fallback chain: FN -> N (reconstructed) -> ORG -> EMAIL -> Unknown Contact"
  - "etag field uses #[serde(default)] for backward compatibility with existing contacts"

patterns-established:
  - "vCard mapping returns MappedContact { contact, notes } to separate frontmatter from body"
  - "Dedup matches on source_id (CardDAV UID), not name-based fuzzy matching"

requirements-completed: [SYNC-02, SYNC-03, SYNC-04]

# Metrics
duration: 6min
completed: 2026-03-06
---

# Phase 02 Plan 01: Data Layer Summary

**vCard-to-Contact mapping with calcard, duplicate detection by source_id, and ETag change tracking**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-06T13:47:47Z
- **Completed:** 2026-03-06T13:54:15Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Contact struct extended with etag field for sync metadata (backward compatible via serde default)
- vCard mapping module parses vCard 3.0/4.0 via calcard, maps all key properties to Contact fields
- Duplicate detection module enables source_id matching and ETag-based update decisions
- 19 new tests covering mapping edge cases, name fallback chain, and dedup logic

## Task Commits

Each task was committed atomically:

1. **Task 1: Add etag field to Contact struct, schema, and template** - `e145e26` (feat)
2. **Task 2: Create vcard_map and dedup modules with tests** - `8593955` (feat)

## Files Created/Modified
- `src/models/contact.rs` - Added etag field with serde(default)
- `src/sync/vcard_map.rs` - vCard-to-Contact mapping with name fallback chain
- `src/sync/dedup.rs` - Duplicate detection by source_id and ETag comparison
- `src/sync/mod.rs` - Module exports for vcard_map and dedup
- `.schemas/contact.yaml` - Added etag field to schema
- `templates/contact.md` - Added etag field to template
- `src/validation.rs` - Updated valid_contact() helper, added etag tests
- `src/sync/carddav.rs` - Fixed pre-existing borrow issues with quick-xml temporaries

## Decisions Made
- Used calcard crate (0.3.2) for vCard parsing -- supports both 3.0 and 4.0, from Stalwart Labs
- MappedContact struct separates contact data from notes string (notes go in markdown body, not frontmatter)
- Name fallback: FN -> N (reconstructed as "Given Family") -> ORG -> first EMAIL -> "Unknown Contact"
- etag uses #[serde(default)] so existing contact files without it deserialize with empty string

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing carddav.rs compilation errors**
- **Found during:** Task 2 (creating vcard_map and dedup modules)
- **Issue:** Pre-existing carddav.rs had borrow errors (quick-xml e.name().as_ref() creates dropped temporaries) and local_name returned &str instead of owned String
- **Fix:** Added let bindings for QName temporaries, changed local_name to return owned String
- **Files modified:** src/sync/carddav.rs
- **Verification:** cargo build and cargo test both pass (56 tests)
- **Committed in:** 8593955 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Fix was necessary to unblock compilation. No scope creep.

## Issues Encountered
None beyond the pre-existing carddav.rs compilation issue documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- vcard_map and dedup modules ready for consumption by sync command (02-02)
- Contact struct has all fields needed for sync metadata tracking
- calcard dependency installed and verified working

---
*Phase: 02-carddav-sync*
*Completed: 2026-03-06*
