# AgenticCRM

## What This Is

A plain-text, git-native personal CRM that stores contacts as markdown files with YAML frontmatter. Features a Rust CLI (`acrm`) with full CRUD, iCloud CardDAV sync, and an interactive terminal UI. Designed to be equally usable by humans (via CLI/TUI) and AI agents (via structured files, JSON output, and future MCP).

## Core Value

Your contacts and relationship history are always accessible, portable, and under your control — no vendor lock-in, no cloud dependency, readable by any tool.

## Requirements

### Validated

- ✓ Contact storage as markdown + YAML frontmatter — existing
- ✓ Rust CLI (`acrm`) with add, list, search, show, log, due commands — existing
- ✓ Shell scripts for quick operations and LinkedIn CSV import — existing
- ✓ Contact schema definition and template — existing
- ✓ Git-native version control of all data — existing
- ✓ JSON output mode for all CLI commands (`--format json`) — v1.0
- ✓ Contact editing from CLI (`acrm edit --set key=value`) — v1.0
- ✓ Round-trip serialization preserving YAML comments and field order — v1.0
- ✓ Validation of required fields, enum values, and date formats — v1.0
- ✓ Delete and archive contacts from CLI — v1.0
- ✓ Cadence-based next_follow_up auto-calculation — v1.0
- ✓ iCloud CardDAV pull sync with dedup and vCard mapping — v1.0
- ✓ Sync metadata (source, source_id, ETag) in frontmatter — v1.0
- ✓ Interactive TUI with scrollable contact table — v1.0
- ✓ Split-pane detail view with keyboard navigation — v1.0
- ✓ Real-time search filtering — v1.0
- ✓ Follow-up dashboard with overdue/upcoming contacts — v1.0
- ✓ Log interaction from TUI — v1.0
- ✓ Color-coded status and priority indicators — v1.0

### Active

- [ ] MCP server exposing CRM as tool server for AI agents
- [ ] Two-way iCloud sync (push + conflict resolution)
- [ ] Selective sync by tag/status filter
- [ ] Bulk operations (mass tagging, filtering, pipeline-style ops)
- [ ] LinkedIn connector (richer than current CSV import)

### Out of Scope

- Outlook/Exchange connector — defer to future milestone
- Facebook/X connectors — defer to future milestone
- Web UI — CLI/TUI first, web later
- Cloud hosting / multi-user — this is a personal, local-first tool
- Mobile app — use via terminal or agent integration
- Built-in email client — massive scope; better tools exist
- AI-powered contact enrichment — privacy concerns; unreliable data quality
- Calendar integration — complex (CalDAV/OAuth), tangential to CRM core
- Social media auto-scraping — privacy/TOS violations; brittle scrapers
- Full-text indexing — over-engineering for personal scale (<10K contacts)

## Context

Shipped v1.0 with 4,721 LOC Rust across 3 phases in 2 days.
Tech stack: Rust (edition 2024), clap 4, serde, serde_yaml, chrono, anyhow, ratatui 0.29, reqwest, quick-xml, calcard, keyring.
Flat file architecture — no database, every command reads from disk.
`store.rs` handles all file I/O with `ContactFile` as the primary unit.
Raw frontmatter preservation pattern ensures YAML comments survive editing.

**Known tech debt (3 items):**
- SyncConfig struct unused (dead_code)
- update_existing_contact bypasses store::serialize_contact_file
- 1 dead_code warning in TUI module

## Constraints

- **Tech stack**: Rust for all new features — no runtime dependencies beyond the compiled binary
- **Data format**: Markdown + YAML frontmatter must remain the canonical storage format
- **Sync direction**: CRM is source of truth — CRM wins on all sync conflicts
- **Local-first**: No cloud services required for core functionality
- **Compatibility**: Must not break existing contact files or CLI commands

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| CRM wins on sync conflicts | Simplifies conflict resolution, user controls their data | ✓ Good — implemented in v1.0 |
| ratatui for TUI | De facto Rust TUI framework, active ecosystem | ✓ Good — TEA pattern worked well |
| CardDAV for iCloud | Standard protocol Apple supports, avoids private API | ✓ Good — full discovery chain works |
| MCP for agent integration | Standard protocol for AI tool use, future-proof | — Pending (v2) |
| Raw frontmatter preservation | YAML comments and field order survive round-trip editing | ✓ Good — key differentiator |
| reqwest blocking (not async) | Avoids tokio complexity for CLI tool | ✓ Good — simpler codebase |
| TEA pattern for TUI | Predictable state management with Screen/InputMode/Message enums | ✓ Good — clean state transitions |
| calcard for vCard parsing | Production quality, supports 3.0 and 4.0 | ✓ Good — handles iCloud vCards |
| Dedup by source_id not name | Exact matching avoids false positives | ✓ Good — reliable sync |

---
*Last updated: 2026-03-06 after v1.0 milestone*
