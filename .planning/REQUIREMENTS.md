# Requirements: AgenticCRM

**Defined:** 2026-03-07
**Core Value:** Your contacts and relationship history are always accessible, portable, and under your control

## v1.1 Requirements

Requirements for two-way iCloud sync milestone. Each maps to roadmap phases.

### Push Infrastructure

- [x] **PUSH-01**: User can push a new CRM contact to iCloud via `acrm sync push` (creates vCard on server)
- [x] **PUSH-02**: User can push updated CRM contact to iCloud (replaces vCard on server)
- [x] **PUSH-03**: User can push CRM deletion/archive to iCloud (removes contact from server)
- [x] **PUSH-04**: Push preserves iCloud data not mapped to CRM (photos, TYPE params, X-properties) via vCard cache
- [x] **PUSH-05**: User sees conflict warning when iCloud has a newer version (CRM wins by default)

### Push Command

- [ ] **CMD-01**: User can run `acrm sync push` to push all local changes to iCloud
- [ ] **CMD-02**: User can run `acrm sync push --dry-run` to preview changes without pushing
- [ ] **CMD-03**: User can run `acrm sync push --force` to skip conflict checks
- [ ] **CMD-04**: Push reports summary (X created, Y updated, Z deleted, W conflicts)

### Selective Sync

- [ ] **FILT-01**: User can configure push tag/status filters in sync config
- [ ] **FILT-02**: User can configure pull tag/status filters in sync config
- [ ] **FILT-03**: User can override filters via `--tag` and `--status` CLI flags
- [ ] **FILT-04**: Default (no filters) syncs everything

### Bidirectional Sync

- [ ] **BIDI-01**: `acrm sync` performs pull-then-push in one command
- [ ] **BIDI-02**: User can still run `acrm sync pull` and `acrm sync push` separately

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Agent Integration

- **MCP-01**: MCP server exposes CRM as tool server for AI agents
- **MCP-02**: AI agents can search, read, and update contacts via MCP

### Auto-Push

- **AUTO-01**: Optional `--push` flag on edit/log commands triggers immediate sync
- **AUTO-02**: Auto-push respects selective sync filters

### Bulk Operations

- **BULK-01**: Mass tagging of contacts by filter
- **BULK-02**: Pipeline-style bulk operations

### Connectors

- **CONN-01**: Richer LinkedIn connector (beyond CSV import)

## Out of Scope

| Feature | Reason |
|---------|--------|
| File watcher daemon (`acrm sync watch`) | Over-complex for personal tool; simpler `--push` flag preferred |
| Field-level merge on conflicts | Massive complexity; CRM-wins is sufficient |
| Server-wins conflict mode | Contradicts "CRM is source of truth" constraint |
| Real-time bidirectional sync | Requires persistent connection; Apple doesn't offer webhooks |
| Photo sync | vCard PHOTO is large (base64); CRM has no photo concept |
| vCard group/distribution list sync | Apple-proprietary; complex semantics |
| WebDAV-Sync (RFC 6578) sync-token | Over-optimization at personal scale |
| Async/tokio for HTTP | reqwest blocking is a validated good decision |
| Multi-address-book sync | iCloud typically has one; rare use case |
| CTag quick-check optimization | Useful but not required; add later without breaking changes |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PUSH-01 | Phase 4 | Complete |
| PUSH-02 | Phase 4 | Complete |
| PUSH-03 | Phase 4 | Complete |
| PUSH-04 | Phase 4 | Complete |
| PUSH-05 | Phase 4 | Complete |
| CMD-01 | Phase 5 | Pending |
| CMD-02 | Phase 5 | Pending |
| CMD-03 | Phase 5 | Pending |
| CMD-04 | Phase 5 | Pending |
| FILT-01 | Phase 6 | Pending |
| FILT-02 | Phase 6 | Pending |
| FILT-03 | Phase 6 | Pending |
| FILT-04 | Phase 6 | Pending |
| BIDI-01 | Phase 6 | Pending |
| BIDI-02 | Phase 6 | Pending |

**Coverage:**
- v1.1 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0

---
*Requirements defined: 2026-03-07*
*Last updated: 2026-03-07 after roadmap creation*
