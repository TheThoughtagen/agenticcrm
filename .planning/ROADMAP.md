# Roadmap: AgenticCRM

## Overview

AgenticCRM already has a working Rust CLI with basic contact management. This roadmap extends it with robust CLI editing and validation, iCloud CardDAV sync, and an interactive TUI. The three phases follow natural dependency order: solidify the CLI foundation first, then add network sync capabilities, then build the interactive interface on top.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: CLI Foundation** - Robust editing, validation, JSON output, and data integrity
- [ ] **Phase 2: CardDAV Sync** - Pull contacts from iCloud into CRM with dedup and format conversion
- [ ] **Phase 3: Interactive TUI** - Dashboard and contact browser with ratatui

## Phase Details

### Phase 1: CLI Foundation
**Goal**: Users can confidently edit, validate, and script against their CRM data
**Depends on**: Nothing (first phase)
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06
**Success Criteria** (what must be TRUE):
  1. User can pipe any `acrm` command output to `jq` and get valid JSON
  2. User can update any frontmatter field on a contact without opening the file
  3. User can delete or archive a contact and the operation is reversible (archive) or confirmed (delete)
  4. User sees clear validation errors when required fields are missing or dates are malformed
  5. Logging an interaction with a cadence-configured contact automatically sets the next follow-up date
**Plans**: TBD

Plans:
- [ ] 01-01: TBD
- [ ] 01-02: TBD

### Phase 2: CardDAV Sync
**Goal**: Users can pull their iCloud contacts into the CRM without duplicates
**Depends on**: Phase 1
**Requirements**: SYNC-01, SYNC-02, SYNC-03, SYNC-04
**Success Criteria** (what must be TRUE):
  1. User can run a sync command and see new iCloud contacts appear as markdown files in `contacts/`
  2. Re-running sync does not create duplicate files for contacts already imported
  3. Each synced contact file contains source metadata (iCloud, CardDAV UID, ETag) in frontmatter
  4. vCard fields (name, email, phone, org) are correctly mapped to CRM YAML frontmatter fields
**Plans**: TBD

Plans:
- [ ] 02-01: TBD
- [ ] 02-02: TBD

### Phase 3: Interactive TUI
**Goal**: Users can browse, search, and manage contacts interactively without leaving the terminal
**Depends on**: Phase 1
**Requirements**: TUI-01, TUI-02, TUI-03, TUI-04, TUI-05, TUI-06, TUI-07
**Success Criteria** (what must be TRUE):
  1. User can launch `acrm tui` and see a scrollable contact list with key info columns
  2. User can select a contact and view full details in a split pane without leaving the TUI
  3. User can type `/` to search and see results filter in real-time
  4. User can switch to a follow-up dashboard showing overdue and upcoming contacts
  5. User can log an interaction directly from the TUI without dropping to the CLI
**Plans**: TBD

Plans:
- [ ] 03-01: TBD
- [ ] 03-02: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. CLI Foundation | 0/0 | Not started | - |
| 2. CardDAV Sync | 0/0 | Not started | - |
| 3. Interactive TUI | 0/0 | Not started | - |
