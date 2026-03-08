# Roadmap: AgenticCRM

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- 🚧 **v1.1 Two-Way iCloud Sync** — Phases 4-6 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

- [x] **Phase 1: CLI Foundation** - Frontmatter editing, validation, JSON output, delete/archive (3/3 plans)
- [x] **Phase 2: CardDAV Sync** - iCloud pull sync with PROPFIND discovery, vCard mapping, dedup (3/3 plans)
- [x] **Phase 3: Interactive TUI** - Contact browser, detail view, search, follow-up dashboard (3/3 plans)

</details>

### v1.1 Two-Way iCloud Sync

- [ ] **Phase 4: Push Infrastructure** - vCard serialization, CardDAV PUT/DELETE, vCard cache, ETag conflict detection
- [ ] **Phase 5: Push Command** - `acrm sync push` with dry-run, force, and result reporting
- [ ] **Phase 6: Selective Sync & Bidirectional** - Tag/status filters for push and pull, `acrm sync` as pull-then-push

## Phase Details

### Phase 4: Push Infrastructure
**Goal**: CRM can serialize contacts to vCard 3.0 and write them to iCloud via CardDAV PUT/DELETE with lossless round-tripping
**Depends on**: Phase 3 (v1.0 complete)
**Requirements**: PUSH-01, PUSH-02, PUSH-03, PUSH-04, PUSH-05
**Success Criteria** (what must be TRUE):
  1. A new CRM contact with no iCloud history can be pushed to iCloud and appears as a contact in iCloud
  2. An updated CRM contact can be pushed to iCloud and the changes appear in iCloud
  3. A deleted/archived CRM contact triggers removal of the corresponding iCloud contact
  4. Pushing a contact back to iCloud preserves iCloud-only data (photos, TYPE params, X-properties) via vCard cache
  5. When iCloud has a newer version (different ETag), the user sees a conflict warning before push proceeds
**Plans**: 3 plans

Plans:
- [ ] 04-01-PLAN.md — vCard serialization and cache module
- [ ] 04-02-PLAN.md — CardDAV PUT/DELETE methods
- [ ] 04-03-PLAN.md — Push orchestration and pull cache integration

### Phase 5: Push Command
**Goal**: User has a complete CLI interface for pushing CRM changes to iCloud with previewing, overriding, and reporting
**Depends on**: Phase 4
**Requirements**: CMD-01, CMD-02, CMD-03, CMD-04
**Success Criteria** (what must be TRUE):
  1. User can run `acrm sync push` and all local changes are pushed to iCloud
  2. User can run `acrm sync push --dry-run` and see what would be pushed without any server changes
  3. User can run `acrm sync push --force` to skip conflict checks and push regardless
  4. After push completes, user sees a summary reporting counts of created, updated, deleted, and conflicted contacts
**Plans**: 2 plans

Plans:
- [x] 05-01-PLAN.md — Implement execute_push and add sync push/pull CLI subcommands
- [ ] 05-02-PLAN.md — Fix false-positive changeset detection and vCard merge data loss (gap closure)

### Phase 6: Selective Sync & Bidirectional
**Goal**: User can control which contacts sync in each direction and run a single command for full bidirectional sync
**Depends on**: Phase 5
**Requirements**: FILT-01, FILT-02, FILT-03, FILT-04, BIDI-01, BIDI-02
**Success Criteria** (what must be TRUE):
  1. User can configure tag and status filters in sync config so only matching contacts are pushed
  2. User can configure tag and status filters in sync config so only matching contacts are pulled
  3. User can override configured filters via `--tag` and `--status` CLI flags on sync commands
  4. With no filters configured, all contacts sync in both directions (default behavior preserved)
  5. User can run `acrm sync` to perform pull-then-push in a single command, and can still run `acrm sync pull` or `acrm sync push` individually
**Plans**: TBD

Plans:
- [ ] 06-01: TBD
- [ ] 06-02: TBD

## Progress

**Execution Order:** Phases 4 -> 5 -> 6

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. CLI Foundation | v1.0 | 3/3 | Complete | 2026-03-05 |
| 2. CardDAV Sync | v1.0 | 3/3 | Complete | 2026-03-06 |
| 3. Interactive TUI | v1.0 | 3/3 | Complete | 2026-03-06 |
| 4. Push Infrastructure | v1.1 | 0/3 | Not started | - |
| 5. Push Command | v1.1 | 1/2 | In progress | - |
| 6. Selective Sync & Bidirectional | v1.1 | 0/2 | Not started | - |
