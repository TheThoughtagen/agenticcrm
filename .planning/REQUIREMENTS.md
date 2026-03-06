# Requirements: AgenticCRM

**Defined:** 2026-03-05
**Core Value:** Your contacts and relationship history are always accessible, portable, and under your control

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### CLI Foundation

- [x] **CLI-01**: User can get JSON output from any CLI command via `--format json`
- [ ] **CLI-02**: User can edit contact frontmatter fields from CLI (`acrm edit "name" --field value`)
- [x] **CLI-03**: Contact files survive round-trip serialization without losing unknown fields, comments, or field order
- [x] **CLI-04**: CLI validates required fields, enum values, and date formats before writing contact files
- [ ] **CLI-05**: User can delete or archive contacts from CLI
- [ ] **CLI-06**: Logging an interaction auto-calculates `next_follow_up` from `follow_up_cadence`

### CardDAV Sync

- [ ] **SYNC-01**: User can pull contacts from iCloud via CardDAV into the CRM
- [ ] **SYNC-02**: Pulled contacts are converted from vCard format to CRM markdown+YAML format
- [ ] **SYNC-03**: Duplicate detection prevents re-importing contacts that already exist
- [ ] **SYNC-04**: Sync metadata (source, source_id, ETag) is stored in contact frontmatter

### TUI

- [ ] **TUI-01**: User can browse contacts in a scrollable table (name, company, status, last contacted)
- [ ] **TUI-02**: User can view contact details in a split-pane layout
- [ ] **TUI-03**: User can navigate with keyboard shortcuts (vim-style j/k, search with /)
- [ ] **TUI-04**: User can see a follow-up dashboard showing overdue and upcoming contacts
- [ ] **TUI-05**: User can search and filter contacts in real-time from the TUI
- [ ] **TUI-06**: User can log an interaction directly from the TUI
- [ ] **TUI-07**: TUI displays color-coded priority and status indicators

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### MCP Server

- **MCP-01**: MCP server exposes search, get, add, update, log, due tools
- **MCP-02**: MCP server uses JSON-RPC 2.0 over stdio

### CardDAV Push/Sync

- **SYNC-05**: Push CRM changes to iCloud
- **SYNC-06**: Two-way sync with CRM-wins conflict resolution
- **SYNC-07**: Selective sync by tag/status filter

### CLI Enhancements

- **CLI-07**: Bulk tag/untag operations with filter-based selection
- **CLI-08**: Multi-criteria filtering and sort options for list command
- **CLI-09**: Contact merge/dedup with matching heuristics

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web UI | CLI/TUI-first philosophy; scope creep |
| Cloud hosting / multi-user | Personal, local-first tool by design |
| Built-in email client | Massive scope; better tools exist |
| AI-powered contact enrichment | Privacy concerns; unreliable data quality |
| Calendar integration | Complex (CalDAV/OAuth), tangential to CRM core |
| Social media auto-scraping | Privacy/TOS violations; brittle scrapers |
| Mobile app | Terminal apps work via SSH/Termux |
| Notification system | Pull-based (acrm due) is sufficient; agents handle notifications |
| Full-text indexing | Over-engineering for personal scale (<10K contacts) |
| Outlook/Exchange connector | Defer to future milestone |
| Facebook/X connectors | Defer to future milestone |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CLI-01 | Phase 1 | Complete |
| CLI-02 | Phase 1 | Pending |
| CLI-03 | Phase 1 | Complete |
| CLI-04 | Phase 1 | Complete |
| CLI-05 | Phase 1 | Pending |
| CLI-06 | Phase 1 | Pending |
| SYNC-01 | Phase 2 | Pending |
| SYNC-02 | Phase 2 | Pending |
| SYNC-03 | Phase 2 | Pending |
| SYNC-04 | Phase 2 | Pending |
| TUI-01 | Phase 3 | Pending |
| TUI-02 | Phase 3 | Pending |
| TUI-03 | Phase 3 | Pending |
| TUI-04 | Phase 3 | Pending |
| TUI-05 | Phase 3 | Pending |
| TUI-06 | Phase 3 | Pending |
| TUI-07 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 17 total
- Mapped to phases: 17
- Unmapped: 0

---
*Requirements defined: 2026-03-05*
*Last updated: 2026-03-05 after roadmap creation*
