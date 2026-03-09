# Roadmap: AgenticCRM

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- ✅ **v1.1 Two-Way iCloud Sync** — Phases 4-6 (shipped 2026-03-08)
- 🚧 **v1.2 MCP, Bulk Ops & LinkedIn** — Phases 7-10 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

- [x] **Phase 1: CLI Foundation** - Frontmatter editing, validation, JSON output, delete/archive (3/3 plans)
- [x] **Phase 2: CardDAV Sync** - iCloud pull sync with PROPFIND discovery, vCard mapping, dedup (3/3 plans)
- [x] **Phase 3: Interactive TUI** - Contact browser, detail view, search, follow-up dashboard (3/3 plans)

</details>

<details>
<summary>✅ v1.1 Two-Way iCloud Sync (Phases 4-6) — SHIPPED 2026-03-08</summary>

- [x] **Phase 4: Push Infrastructure** - vCard serialization, CardDAV PUT/DELETE, vCard cache, ETag conflict detection
- [x] **Phase 5: Push Command** - `acrm sync push` with dry-run, force, and result reporting
- [x] **Phase 6: Selective Sync & Bidirectional** - Tag/status filters for push and pull, `acrm sync` as pull-then-push

</details>

### v1.2 MCP, Bulk Ops & LinkedIn

- [x] **Phase 7: Operations Layer** - Extract business logic from CLI into shared ops module (completed 2026-03-09)
- [ ] **Phase 8: Bulk Operations & Query Engine** - Query syntax, bulk edit/delete/archive/tag, dry-run, JSON pipe
- [ ] **Phase 9: MCP Server** - `acrm serve` with stdio and Streamable HTTP transports, full CRM tools for AI agents
- [ ] **Phase 10: LinkedIn Import** - Rust-native CSV import with dedup, change detection, and field mapping

## Phase Details

### Phase 7: Operations Layer
**Goal**: All CRM business logic lives in a shared ops module that both CLI and future consumers call directly
**Depends on**: Phase 6 (v1.1 complete)
**Requirements**: OPS-01, OPS-02
**Success Criteria** (what must be TRUE):
  1. Every CLI command (add, list, search, show, edit, log, due, delete, archive) delegates to a function in the ops module
  2. All existing CLI commands produce identical output and behavior before and after the refactor
  3. The ops module functions accept plain arguments and return `Result<T>` -- no CLI or clap types leak into ops
**Plans**: 2 plans

Plans:
- [ ] 07-01-PLAN.md — Create ops module and extract CRUD business logic from CLI handlers
- [ ] 07-02-PLAN.md — Extract sync operations, wire TUI to ops, zero compiler warnings

### Phase 8: Bulk Operations & Query Engine
**Goal**: Users can query contacts with field predicates and apply bulk changes in a single command, with safety guards and Unix composability
**Depends on**: Phase 7
**Requirements**: BULK-01, BULK-02, BULK-03, BULK-04, BULK-05, BULK-06, BULK-07
**Success Criteria** (what must be TRUE):
  1. User can run `acrm bulk 'status=dormant'` and see all contacts matching the query
  2. User can bulk update, delete, archive, or tag matched contacts in one command (`--set`, `--delete`, `--archive`, `--add-tag`, `--remove-tag`)
  3. Bulk operations show a preview and require confirmation before making changes (skippable with `--yes`)
  4. User can run any bulk command with `--dry-run` to see what would change without writing to disk
  5. User can pipe JSON output from `acrm search` into `acrm bulk-update --stdin` for Unix-style composition
**Plans**: 2 plans

Plans:
- [x] 08-01-PLAN.md — Query engine (predicate parser + matcher) and bulk ops functions in ops layer
- [ ] 08-02-PLAN.md — CLI bulk/bulk-update commands with preview/confirm, dry-run, and stdin JSON pipe

### Phase 9: MCP Server
**Goal**: AI agents can discover and use all CRM operations as MCP tools via `acrm serve`, with safe concurrent access
**Depends on**: Phase 8
**Requirements**: MCP-01, MCP-02, MCP-03, MCP-04, MCP-05, MCP-06, MCP-07, MCP-08, MCP-09, MCP-10, MCP-11, MCP-12
**Success Criteria** (what must be TRUE):
  1. Agent can connect to `acrm serve` over stdio and discover all CRM tools (search, show, add, edit, log, delete, archive, due, sync)
  2. Agent can connect to `acrm serve --http` over Streamable HTTP for remote access
  3. Agent can perform full read+write CRM operations (search, view, add, edit, log interaction, delete/archive, list due follow-ups, trigger sync) through MCP tools
  4. Contacts are browsable as MCP resources via `contact://` URIs
  5. Concurrent MCP requests from the same or multiple agents do not corrupt contact files
**Plans**: TBD

Plans:
- [ ] 09-01: TBD
- [ ] 09-02: TBD
- [ ] 09-03: TBD

### Phase 10: LinkedIn Import
**Goal**: Users can import LinkedIn connection data into the CRM with intelligent deduplication and change detection
**Depends on**: Phase 7 (uses ops layer; independent of Phases 8-9)
**Requirements**: LNKD-01, LNKD-02, LNKD-03, LNKD-04
**Success Criteria** (what must be TRUE):
  1. User can run `acrm import linkedin <file>` and contacts from the CSV are created in the CRM
  2. Re-importing the same CSV does not create duplicates (matched by name and email)
  3. Re-importing an updated CSV detects and applies only changed fields, leaving manually-edited CRM fields intact
  4. All available LinkedIn CSV columns (first name, last name, email, company, position, connected on) are mapped to the contact schema
**Plans**: TBD

Plans:
- [ ] 10-01: TBD
- [ ] 10-02: TBD

## Progress

**Execution Order:** Phases 7 -> 8 -> 9 -> 10

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. CLI Foundation | v1.0 | 3/3 | Complete | 2026-03-05 |
| 2. CardDAV Sync | v1.0 | 3/3 | Complete | 2026-03-06 |
| 3. Interactive TUI | v1.0 | 3/3 | Complete | 2026-03-06 |
| 4. Push Infrastructure | v1.1 | 3/3 | Complete | 2026-03-07 |
| 5. Push Command | v1.1 | 2/2 | Complete | 2026-03-08 |
| 6. Selective Sync & Bidirectional | v1.1 | 2/2 | Complete | 2026-03-08 |
| 7. Operations Layer | 2/2 | Complete   | 2026-03-09 | - |
| 8. Bulk Operations & Query Engine | v1.2 | 1/2 | In progress | - |
| 9. MCP Server | v1.2 | 0/? | Not started | - |
| 10. LinkedIn Import | v1.2 | 0/? | Not started | - |
