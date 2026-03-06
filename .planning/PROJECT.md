# AgenticCRM

## What This Is

A plain-text, git-native personal CRM that stores contacts as markdown files with YAML frontmatter. Designed to be equally usable by humans (via CLI/TUI) and AI agents (via structured files, JSON output, and MCP). Currently has a Rust CLI (`acrm`) with basic commands and shell script utilities.

## Core Value

Your contacts and relationship history are always accessible, portable, and under your control — no vendor lock-in, no cloud dependency, readable by any tool.

## Requirements

### Validated

- ✓ Contact storage as markdown + YAML frontmatter — existing
- ✓ Rust CLI (`acrm`) with add, list, search, show, log, due commands — existing
- ✓ Shell scripts for quick operations and LinkedIn CSV import — existing
- ✓ Contact schema definition and template — existing
- ✓ Git-native version control of all data — existing

### Active

- [ ] Two-way iCloud/Apple Contacts sync via CardDAV (CRM wins on conflicts)
- [ ] LinkedIn connector (richer than current CSV import)
- [ ] Interactive TUI with ratatui (dashboard + contact browser)
- [ ] JSON output mode for all CLI commands
- [ ] MCP server exposing CRM as tool server for AI agents
- [ ] Bulk operations (mass tagging, filtering, pipeline-style ops)
- [ ] Contact editing from CLI (update frontmatter fields)

### Out of Scope

- Outlook/Exchange connector — defer to future milestone
- Facebook/X connectors — defer to future milestone
- Web UI — CLI/TUI first, web later
- Cloud hosting / multi-user — this is a personal, local-first tool
- Mobile app — use via terminal or agent integration

## Context

- Rust edition 2024, using clap 4, serde, serde_yaml, chrono, anyhow
- Flat file architecture — no database, every command reads from disk
- `store.rs` handles all file I/O with `ContactFile` as the primary unit
- Shell scripts exist as legacy/utility interface alongside the Rust CLI
- Schema at `.schemas/contact.yaml` is documentation-only, not enforced at runtime
- iCloud sync will require CardDAV protocol (RFC 6352) via `vdirsyncer` or native implementation
- LinkedIn export provides CSV; richer integration may need scraping or API workarounds

## Constraints

- **Tech stack**: Rust for all new features — no runtime dependencies beyond the compiled binary
- **Data format**: Markdown + YAML frontmatter must remain the canonical storage format
- **Sync direction**: CRM is source of truth — CRM wins on all sync conflicts
- **Local-first**: No cloud services required for core functionality
- **Compatibility**: Must not break existing contact files or CLI commands

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| CRM wins on sync conflicts | Simplifies conflict resolution, user controls their data | — Pending |
| ratatui for TUI | De facto Rust TUI framework, active ecosystem | — Pending |
| MCP for agent integration | Standard protocol for AI tool use, future-proof | — Pending |
| CardDAV for iCloud | Standard protocol Apple supports, avoids private API | — Pending |

---
*Last updated: 2026-03-05 after initialization*
