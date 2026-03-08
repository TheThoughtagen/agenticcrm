---
phase: 05-push-command
plan: 02
subsystem: sync
tags: [vcard, push, carddav, changeset-detection, semantic-comparison]

requires:
  - phase: 05-push-command/01
    provides: "Push command infrastructure (compute_push_changeset, execute_push, vcard_write)"
provides:
  - "Semantic Contact-field comparison for push changeset detection (ContactSnapshot)"
  - "In-place vCard merge preserving EMAIL/TEL params, NOTE, and property ordering"
  - "Contact snapshot caching on pull and push operations"
affects: [push-command, sync, uat]

tech-stack:
  added: []
  patterns: [contact-snapshot-caching, semantic-field-comparison, in-place-vcard-merge]

key-files:
  created: []
  modified:
    - src/sync/vcard_write.rs
    - src/sync/push.rs
    - src/commands/sync.rs

key-decisions:
  - "Semantic comparison via ContactSnapshot JSON files instead of vCard string comparison"
  - "Remove NOTE from CRM_MAPPED_PROPERTIES since Contact model has no notes field"
  - "In-place vCard property replacement preserving original ordering and groups"
  - "EMAIL/TEL params preserved by value-matching against cached entries"

patterns-established:
  - "ContactSnapshot: JSON cache of CRM-relevant fields for semantic change detection"
  - "In-place merge: walk cached vCard entries replacing CRM props at original positions"

requirements-completed: [CMD-01, CMD-02]

duration: 6min
completed: 2026-03-08
---

# Phase 5 Plan 02: Gap Closure Summary

**Semantic push changeset detection and lossless vCard merge fixing 985 false-positive updates**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-08T18:15:53Z
- **Completed:** 2026-03-08T18:21:40Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Replaced string-based vCard comparison with semantic ContactSnapshot comparison, eliminating ~985 false-positive updates in push dry-run
- Rewrote merge_contact_to_vcard for in-place property replacement preserving EMAIL/TEL TYPE parameters, NOTE property, and original property ordering
- Fixed N encoding inconsistency between contact_to_vcard (VCardValue::Text) and merge path (was VCardValue::Component)
- Added contact snapshot caching in pull and push paths for round-trip consistency

## Task Commits

Each task was committed atomically:

1. **Task 1: Add semantic Contact-field comparison for changeset detection** - `0f86d8a` (feat)
2. **Task 2: Fix vCard merge to preserve params and include NOTE** - `76f3c55` (fix)

## Files Created/Modified
- `src/sync/vcard_write.rs` - ContactSnapshot struct/cache functions, rewritten merge_contact_to_vcard with in-place replacement and param preservation, removed dead CRM_MAPPED_PROPERTIES const and add_crm_entries function
- `src/sync/push.rs` - Replaced string comparison with contact_fields_changed semantic check, added snapshot caching after create/update/force-push operations
- `src/commands/sync.rs` - Added contact snapshot caching after pull create/update operations

## Decisions Made
- Used ContactSnapshot JSON files (not vCard string comparison) for change detection -- avoids all serialization-related false positives
- Removed NOTE from CRM_MAPPED_PROPERTIES since the Contact model has no notes field; stripping NOTE destroys data with no way to re-add it
- In-place merge walks cached vCard entries in order, replacing CRM properties at their original positions rather than stripping and appending
- EMAIL/TEL params are preserved by matching values: if an email value exists in both cached and contact, the cached entry's params are copied

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Push dry-run now correctly reports only genuinely changed contacts
- vCard merge preserves server-side data (TYPE params, NOTE, property ordering)
- Ready for UAT testing of push operations

---
*Phase: 05-push-command*
*Completed: 2026-03-08*

## Self-Check: PASSED
