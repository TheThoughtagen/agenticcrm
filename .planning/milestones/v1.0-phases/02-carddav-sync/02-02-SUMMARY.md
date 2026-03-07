---
phase: 02-carddav-sync
plan: 02
subsystem: sync
tags: [carddav, webdav, reqwest, quick-xml, keyring, icloud, xml-parsing]

# Dependency graph
requires:
  - phase: 01-cli-foundation
    provides: "Contact model, store module, CLI structure"
provides:
  - "CardDavClient with PROPFIND discovery chain and vCard fetching"
  - "Credential storage via macOS Keychain (keyring)"
  - "SyncConfig with apple_id config file management"
affects: [02-carddav-sync]

# Tech tracking
tech-stack:
  added: [reqwest 0.13 (blocking), quick-xml 0.39, keyring 3.6, url 2.5]
  patterns: [event-based XML parsing with quick-xml Reader, macOS Keychain credential storage]

key-files:
  created:
    - src/sync/carddav.rs
    - src/sync/config.rs
    - src/sync/mod.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/main.rs
    - src/commands/add.rs

key-decisions:
  - "Used reqwest blocking client (not async) to avoid tokio complexity in CLI tool"
  - "Event-based XML parsing with quick-xml Reader instead of serde derive for WebDAV namespace handling"
  - "local_name helper returns owned String to avoid temporary lifetime issues with quick-xml 0.39"
  - "ETag quotes stripped during parsing for clean comparison"

patterns-established:
  - "CardDAV discovery chain: PROPFIND principal -> home-set -> addressbook collection"
  - "XML parsing pattern: event loop with state flags for nested element tracking"
  - "Credential split: apple_id in config file, password in macOS Keychain only"

requirements-completed: [SYNC-01]

# Metrics
duration: 5min
completed: 2026-03-06
---

# Phase 02 Plan 02: CardDAV Client Summary

**CardDAV protocol client with 3-step PROPFIND discovery, vCard listing/fetching, and Keychain credential storage using reqwest + quick-xml**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-06T13:47:51Z
- **Completed:** 2026-03-06T13:52:49Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- CardDAV client with full iCloud discovery chain (principal -> addressbook-home-set -> addressbook collection)
- vCard list fetching with ETag change detection support
- Credential management via macOS Keychain (keyring) with config file for apple_id
- 12 unit tests covering all XML parsing functions and config parsing

## Task Commits

Each task was committed atomically:

1. **Task 1: Add network dependencies and create config module** - `f173a1e` (feat)
2. **Task 2: Create CardDAV client module** - `b280ec6` (feat)

## Files Created/Modified
- `src/sync/carddav.rs` - CardDAV client with PROPFIND discovery, vCard list/fetch, XML parsing helpers
- `src/sync/config.rs` - Credential storage (keyring) and sync config file management
- `src/sync/mod.rs` - Sync module exports (carddav, config)
- `Cargo.toml` - Added reqwest, quick-xml, keyring, url dependencies
- `src/main.rs` - Added sync module declaration
- `src/commands/add.rs` - Fixed missing etag field in Contact initializer

## Decisions Made
- Used reqwest blocking client (not async) to keep CLI tool simple without tokio runtime
- Event-based XML parsing with quick-xml Reader rather than serde derive, as WebDAV XML namespaces are easier to handle with event walking
- local_name helper returns owned String to avoid temporary lifetime issues with quick-xml 0.39 API
- ETag quotes stripped during parse_vcard_entries for clean comparison in later sync logic

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed missing etag field in add command**
- **Found during:** Task 1 (cargo build verification)
- **Issue:** Contact struct has etag field but add.rs did not set it, causing compilation failure
- **Fix:** Added `etag: String::new()` to Contact initializer in add.rs
- **Files modified:** src/commands/add.rs
- **Verification:** cargo build succeeds
- **Committed in:** f173a1e (Task 1 commit)

**2. [Rule 3 - Blocking] Adapted to reqwest 0.13 feature naming**
- **Found during:** Task 1 (dependency installation)
- **Issue:** reqwest 0.13 no longer has `rustls-tls` feature (rustls is now the default)
- **Fix:** Used `--features blocking` instead of `--features rustls-tls`
- **Files modified:** Cargo.toml
- **Verification:** cargo build succeeds
- **Committed in:** f173a1e (Task 1 commit)

**3. [Rule 3 - Blocking] Adapted to quick-xml 0.39 API changes**
- **Found during:** Task 2 (compilation)
- **Issue:** BytesText no longer has unescape() method in quick-xml 0.39; QName temporaries cause lifetime issues
- **Fix:** Used xml_content() instead of unescape(); changed local_name to return owned String
- **Files modified:** src/sync/carddav.rs
- **Verification:** All 8 carddav tests pass
- **Committed in:** b280ec6 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All auto-fixes were necessary to adapt to current library versions. No scope creep.

## Issues Encountered
None beyond the library version adaptations documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CardDAV client ready for integration with sync command
- Next plans will need: vCard parsing (calcard), field mapping, dedup logic, and CLI sync command
- Real iCloud testing will require app-specific password setup

---
*Phase: 02-carddav-sync*
*Completed: 2026-03-06*
