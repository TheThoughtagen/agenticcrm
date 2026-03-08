---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Two-Way iCloud Sync
status: unknown
last_updated: "2026-03-08T20:04:43.758Z"
progress:
  total_phases: 3
  completed_phases: 3
  total_plans: 7
  completed_plans: 7
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-07)

**Core value:** Your contacts and relationship history are always accessible, portable, and under your control
**Current focus:** Phase 6 - Selective Sync & Bidirectional (filtering complete, bidirectional next)

## Current Position

Phase: 6 of 6 (Selective Sync & Bidirectional) -- third phase of v1.1
Plan: 2 of 2 in current phase (Plan 02 complete -- bidirectional sync)
Status: Complete
Last activity: 2026-03-08 -- completed 06-02 bidirectional sync (pull-then-push)

Progress: [██████████] 100% (7/7 v1.1 plans)

## Performance Metrics

**Velocity (from v1.0):**
- Total plans completed: 10
- Average duration: 4 min/plan
- Total execution time: 0.52 hours

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.0: CRM wins on sync conflicts (carries forward with warn + override)
- v1.0: reqwest blocking client for CardDAV (no async/tokio)
- v1.0: Dedup by source_id not name
- v1.0: calcard for vCard parsing
- v1.1: Sync trigger = manual push + optional auto-push config
- v1.1: Conflict resolution = warn + override (CRM still wins)
- v1.1: Empty string returned when server omits ETag in PUT response (caller PROPFINDs)
- v1.1: 200ms sleep before PUT/DELETE as iCloud rate-limit defense
- v1.1: DELETE returns Ok on 404 for idempotent semantics
- v1.1: Use VCardValue::Text (not Component) for N property to get semicolon separators
- v1.1: Merge path (merge_contact_to_vcard) does in-place replacement preserving params and NOTE
- v1.1: ContactSnapshot JSON caching for semantic push changeset detection (replaces string comparison)
- v1.1: Changeset compute-then-execute pattern enables dry-run and preview
- v1.1: CLI flags on both parent and subcommand level, merged via OR for flexible usage
- v1.1: PROPFIND fallback when PUT returns empty ETag
- v1.1: Force flag uses server ETag for If-Match on conflicts
- v1.1: CLI filter flags replace (not union) config filter values when provided
- v1.1: New contacts from server always pass pull filter (no tag filtering on new)
- v1.1: Push filter applied before changeset; archived deletes unaffected
- v1.1: Status matching via explicit match arms not serde serialization
- v1.1: CLI tag/status flags cloned and applied to both pull and push phases in bidirectional sync

### Pending Todos

None yet.

### Blockers/Concerns

- [Resolved]: vCard cache merge strategy validated -- parse-overlay-serialize with CRM_MAPPED_PROPERTIES works correctly
- [Research]: iCloud rate limits undocumented -- plan for 200ms delays + exponential backoff

## Session Continuity

Last session: 2026-03-08
Stopped at: Completed 06-02-PLAN.md (bidirectional sync: pull-then-push)
Resume file: None
