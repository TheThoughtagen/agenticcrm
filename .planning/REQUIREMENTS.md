# Requirements: AgenticCRM

**Defined:** 2026-03-07
**Core Value:** Your contacts and relationship history are always accessible, portable, and under your control

## v1.1 Requirements (Complete)

<details>
<summary>All 15 requirements complete — shipped 2026-03-08</summary>

### Push Infrastructure

- [x] **PUSH-01**: User can push a new CRM contact to iCloud via `acrm sync push` (creates vCard on server)
- [x] **PUSH-02**: User can push updated CRM contact to iCloud (replaces vCard on server)
- [x] **PUSH-03**: User can push CRM deletion/archive to iCloud (removes contact from server)
- [x] **PUSH-04**: Push preserves iCloud data not mapped to CRM (photos, TYPE params, X-properties) via vCard cache
- [x] **PUSH-05**: User sees conflict warning when iCloud has a newer version (CRM wins by default)

### Push Command

- [x] **CMD-01**: User can run `acrm sync push` to push all local changes to iCloud
- [x] **CMD-02**: User can run `acrm sync push --dry-run` to preview changes without pushing
- [x] **CMD-03**: User can run `acrm sync push --force` to skip conflict checks
- [x] **CMD-04**: Push reports summary (X created, Y updated, Z deleted, W conflicts)

### Selective Sync

- [x] **FILT-01**: User can configure push tag/status filters in sync config
- [x] **FILT-02**: User can configure pull tag/status filters in sync config
- [x] **FILT-03**: User can override filters via `--tag` and `--status` CLI flags
- [x] **FILT-04**: Default (no filters) syncs everything

### Bidirectional Sync

- [x] **BIDI-01**: `acrm sync` performs pull-then-push in one command
- [x] **BIDI-02**: User can still run `acrm sync pull` and `acrm sync push` separately

</details>

## v1.2 Requirements

Requirements for v1.2 milestone (MCP, Bulk Ops & LinkedIn). Each maps to roadmap phases.

### Operations Layer

- [x] **OPS-01**: Business logic extracted from CLI handlers into shared ops module
- [x] **OPS-02**: All existing CLI commands delegate to ops layer (no behavior change)

### MCP Server

- [ ] **MCP-01**: MCP server runs via `acrm serve` with stdio transport
- [ ] **MCP-02**: MCP server supports Streamable HTTP transport for remote access
- [ ] **MCP-03**: Agent can search contacts by name, tag, status, or free text via MCP tool
- [ ] **MCP-04**: Agent can view full contact details via MCP tool
- [ ] **MCP-05**: Agent can add a new contact via MCP tool
- [ ] **MCP-06**: Agent can edit contact fields via MCP tool
- [ ] **MCP-07**: Agent can log an interaction on a contact via MCP tool
- [ ] **MCP-08**: Agent can delete or archive a contact via MCP tool
- [ ] **MCP-09**: Agent can list contacts due for follow-up via MCP tool
- [ ] **MCP-10**: Agent can trigger sync push/pull via MCP tool (configurable permission)
- [ ] **MCP-11**: Contacts exposed as MCP resources with `contact://` URIs
- [ ] **MCP-12**: Concurrent MCP requests don't corrupt contact files

### Bulk Operations

- [x] **BULK-01**: User can query contacts with field-based predicates (`acrm bulk 'status=dormant'`)
- [x] **BULK-02**: User can bulk update fields on matched contacts (`--set field=value`)
- [x] **BULK-03**: User can bulk delete or archive matched contacts
- [x] **BULK-04**: User can bulk add/remove tags on matched contacts
- [ ] **BULK-05**: Bulk operations show preview and require confirmation (or `--yes` to skip)
- [ ] **BULK-06**: All bulk commands support `--dry-run` to preview without changes
- [ ] **BULK-07**: JSON pipe input supported (`acrm search --json | acrm bulk-update --stdin`)

### LinkedIn Import

- [ ] **LNKD-01**: User can import LinkedIn CSV via `acrm import linkedin <file>`
- [ ] **LNKD-02**: Import deduplicates against existing contacts by name and email
- [ ] **LNKD-03**: Re-import detects changes and updates only modified fields
- [ ] **LNKD-04**: Import maps all available LinkedIn CSV fields to contact schema

## Future Requirements

### Playwright LinkedIn Automation

- **LNKD-05**: Playwright script automates LinkedIn CSV export download
- **LNKD-06**: `acrm import linkedin --auto` triggers export + import pipeline

### MCP Enhancements

- **MCP-13**: MCP prompt templates for common CRM workflows
- **MCP-14**: Bulk operations exposed as MCP tools

## Out of Scope

| Feature | Reason |
|---------|--------|
| LinkedIn profile scraping | TOS risk, fragile DOM selectors; CSV export is safer path |
| MCP prompt templates | Differentiator but not table stakes; defer to v1.3 |
| HTTP+SSE transport | Deprecated in MCP spec (March 2025); use Streamable HTTP instead |
| Async migration of full codebase | Only `acrm serve` needs tokio; keep CLI synchronous |
| Database/indexing for bulk queries | Flat file scan is sufficient for personal scale (<10K contacts) |
| File watcher daemon (`acrm sync watch`) | Over-complex for personal tool |
| Field-level merge on conflicts | Massive complexity; CRM-wins is sufficient |
| Photo sync | vCard PHOTO is large (base64); CRM has no photo concept |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PUSH-01 | Phase 4 | Complete |
| PUSH-02 | Phase 4 | Complete |
| PUSH-03 | Phase 4 | Complete |
| PUSH-04 | Phase 4 | Complete |
| PUSH-05 | Phase 4 | Complete |
| CMD-01 | Phase 5 | Complete |
| CMD-02 | Phase 5 | Complete |
| CMD-03 | Phase 5 | Complete |
| CMD-04 | Phase 5 | Complete |
| FILT-01 | Phase 6 | Complete |
| FILT-02 | Phase 6 | Complete |
| FILT-03 | Phase 6 | Complete |
| FILT-04 | Phase 6 | Complete |
| BIDI-01 | Phase 6 | Complete |
| BIDI-02 | Phase 6 | Complete |
| OPS-01 | Phase 7 | Complete |
| OPS-02 | Phase 7 | Complete |
| MCP-01 | Phase 9 | Pending |
| MCP-02 | Phase 9 | Pending |
| MCP-03 | Phase 9 | Pending |
| MCP-04 | Phase 9 | Pending |
| MCP-05 | Phase 9 | Pending |
| MCP-06 | Phase 9 | Pending |
| MCP-07 | Phase 9 | Pending |
| MCP-08 | Phase 9 | Pending |
| MCP-09 | Phase 9 | Pending |
| MCP-10 | Phase 9 | Pending |
| MCP-11 | Phase 9 | Pending |
| MCP-12 | Phase 9 | Pending |
| BULK-01 | Phase 8 | Complete |
| BULK-02 | Phase 8 | Complete |
| BULK-03 | Phase 8 | Complete |
| BULK-04 | Phase 8 | Complete |
| BULK-05 | Phase 8 | Pending |
| BULK-06 | Phase 8 | Pending |
| BULK-07 | Phase 8 | Pending |
| LNKD-01 | Phase 10 | Pending |
| LNKD-02 | Phase 10 | Pending |
| LNKD-03 | Phase 10 | Pending |
| LNKD-04 | Phase 10 | Pending |

**Coverage:**
- v1.1 requirements: 15 total (all complete)
- v1.2 requirements: 25 total
- Mapped to phases: 25/25
- Unmapped: 0

---
*Requirements defined: 2026-03-07*
*Last updated: 2026-03-09 after v1.2 roadmap creation*
