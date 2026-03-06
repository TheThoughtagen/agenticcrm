---
phase: 02-carddav-sync
plan: 03
subsystem: sync
tags: [carddav, icloud, cli, vcard, dedup, keychain]

# Dependency graph
requires:
  - phase: 02-01
    provides: "vCard mapping (map_vcard_to_contact) and dedup (find_existing_by_source_id, should_update)"
  - phase: 02-02
    provides: "CardDAV client (CardDavClient) and credential storage (config::store_credentials, load_credentials)"
  - phase: 01-01
    provides: "Frontmatter editor, store module, OutputFormat pattern"
provides:
  - "Working `acrm sync` command that pulls iCloud contacts into markdown files"
  - "Working `acrm sync setup` command for credential configuration"
  - "`--dry-run` and `--force` flags for sync control"
  - "JSON output support for sync results"
affects: [03-interactive-tui]

# Tech tracking
tech-stack:
  added: [dialoguer]
  patterns: [sync-integration-layer, graceful-per-vcard-error-handling]

key-files:
  created:
    - src/commands/sync.rs
  modified:
    - src/commands/mod.rs
    - src/main.rs

key-decisions:
  - "UID extracted from vCard href path (last segment minus .vcf) rather than parsing vCard UID property"
  - "Per-vCard error handling: failed fetch/parse logs warning and continues, does not abort sync"
  - "Update flow writes directly to existing file path (same pattern as 01-03 log command)"
  - "New contacts use generate_raw_frontmatter + frontmatter::update_field/update_array_field for template preservation"

patterns-established:
  - "Sync integration: fetch list -> iterate with per-item error tolerance -> dedup check -> create/update/skip"
  - "Checkpoint:human-verify for real-service integration testing"

requirements-completed: [SYNC-01, SYNC-02, SYNC-03, SYNC-04]

# Metrics
duration: 5min
completed: 2026-03-06
---

# Phase 2 Plan 3: Sync CLI Command Summary

**End-to-end `acrm sync` command wiring CardDAV client, vCard mapping, and dedup into a single CLI flow with dry-run and force modes**

## Performance

- **Duration:** ~5 min (excludes human verification wait time)
- **Started:** 2026-03-06T13:58:05Z
- **Completed:** 2026-03-06T14:17:00Z
- **Tasks:** 2 (1 auto + 1 human-verify)
- **Files modified:** 3

## Accomplishments
- Wired all Phase 2 modules (CardDavClient, vcard_map, dedup, config) into a working `acrm sync` command
- Added `acrm sync setup` for interactive iCloud credential configuration (Apple ID + app-specific password)
- Implemented dry-run mode (`--dry-run`) that previews sync without writing files
- Implemented force mode (`--force`) to re-download all contacts regardless of ETags
- Per-vCard error tolerance: failed fetches/parses log warnings and continue
- Human-verified end-to-end with real iCloud account

## Task Commits

Each task was committed atomically:

1. **Task 1: Create sync CLI command and wire everything together** - `2db72bc` (feat)
2. **Task 2: Verify end-to-end iCloud sync** - checkpoint:human-verify (PASSED)

## Files Created/Modified
- `src/commands/sync.rs` - Sync CLI command with run_setup and run_sync functions, SyncResult output type
- `src/commands/mod.rs` - Added `pub mod sync` export
- `src/main.rs` - Added Sync subcommand with SyncAction::Setup, --force, --dry-run flags

## Decisions Made
- UID extracted from vCard href path (last segment minus .vcf) rather than parsing vCard UID property -- simpler and works with iCloud's URL structure
- Per-vCard error handling logs warning and continues (does not abort entire sync for one bad vCard)
- Update flow writes directly to existing file path, same pattern established in 01-03
- New contacts use template-based frontmatter generation with field updates for iCloud-specific fields

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

iCloud sync requires an app-specific password:
1. Visit appleid.apple.com -> Sign-In and Security -> App-Specific Passwords
2. Generate a new password for "acrm"
3. Run `acrm sync setup` and enter Apple ID + app-specific password
4. Run `acrm sync` to pull contacts

## Next Phase Readiness
- Phase 2 (CardDAV Sync) is fully complete -- all 3 plans done
- All SYNC requirements fulfilled
- Phase 3 (Interactive TUI) can proceed (depends on Phase 1 only)
- iCloud authentication blocker resolved via human verification

---
*Phase: 02-carddav-sync*
*Completed: 2026-03-06*
