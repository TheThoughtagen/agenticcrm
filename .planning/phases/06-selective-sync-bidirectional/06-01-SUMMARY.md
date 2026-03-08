---
phase: 06-selective-sync-bidirectional
plan: 01
subsystem: sync
tags: [toml, filtering, cli, carddav, sync]

requires:
  - phase: 05-push-command
    provides: "Push/pull sync infrastructure with changeset computation"
provides:
  - "SyncFilter module with tag/status matching and config+CLI merge"
  - "Extended SyncConfig with TOML-based push_filters/pull_filters sections"
  - "CLI --tag and --status flags on sync, push, and pull subcommands"
  - "Filter application in push and pull paths"
affects: [06-02-bidirectional-sync]

tech-stack:
  added: [toml 0.8]
  patterns: [config-and-cli-override, retain-filter-pattern]

key-files:
  created:
    - src/sync/filter.rs
  modified:
    - Cargo.toml
    - src/sync/config.rs
    - src/sync/mod.rs
    - src/main.rs
    - src/commands/sync.rs

key-decisions:
  - "CLI flags replace (not union) config filter values when provided"
  - "New contacts from server always pass pull filter (no tag filtering on new)"
  - "Push filter applied before changeset computation; archived deletes unaffected"
  - "Status matching uses explicit match arms not serde serialization"

patterns-established:
  - "Filter merge: CLI overrides config per-dimension (tags, statuses independently)"
  - "Parent+subcommand CLI flag merging with dedup via merge_vecs helper"

requirements-completed: [FILT-01, FILT-02, FILT-03, FILT-04]

duration: 3min
completed: 2026-03-08
---

# Phase 6 Plan 1: Selective Sync Filtering Summary

**Tag/status filtering for push and pull via sync.toml config sections and CLI --tag/--status overrides using toml crate**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-08T19:54:24Z
- **Completed:** 2026-03-08T19:57:37Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- SyncFilter module with matches(), is_empty(), and from_config_and_cli() for tag/status filtering
- SyncConfig extended with serde Deserialize and optional push_filters/pull_filters TOML sections (backward compatible)
- CLI --tag and --status flags on sync, push, and pull subcommands with parent+subcommand merging
- Push path filters active contacts before changeset computation (archived deletes unaffected)
- Pull path skips non-matching existing contacts during update; new contacts always come through

## Task Commits

Each task was committed atomically:

1. **Task 1: Add toml crate, create SyncFilter module, extend SyncConfig** - `e9513f0` (feat)
2. **Task 2: Add CLI flags and wire filters into push and pull paths** - `a6fed9a` (feat)

## Files Created/Modified
- `Cargo.toml` - Added toml 0.8 dependency
- `src/sync/filter.rs` - SyncFilter struct with matches/from_config_and_cli, 11 unit tests
- `src/sync/config.rs` - SyncConfig with serde Deserialize, FilterConfig, load_sync_config(), 3 new tests
- `src/sync/mod.rs` - Added filter module declaration
- `src/main.rs` - CLI --tag/--status flags, filter construction, merge_vecs helper
- `src/commands/sync.rs` - Filter application in run_push and run_sync paths

## Decisions Made
- CLI flags replace (not union with) config filter values when provided -- simpler mental model
- New contacts from server always pass pull filter -- they have no CRM tags yet
- Push filter applied before changeset computation; archived deletes are independently handled by compute_push_changeset scanning archive/ directory
- Status matching uses explicit match arms (Active->active, LostTouch->lost-touch, etc.) rather than serde serialization -- more explicit, no extra dependency in filter logic

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Filter infrastructure ready for Plan 02 (bidirectional sync)
- SyncFilter is passed through to run_sync/run_push, ready for bidirectional wiring
- Config parsing supports filter sections for both directions

---
*Phase: 06-selective-sync-bidirectional*
*Completed: 2026-03-08*
