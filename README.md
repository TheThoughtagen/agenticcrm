# AgenticCRM

A plain-text, agent-friendly personal CRM. Contacts are markdown files with YAML frontmatter, stored in git.

## Structure

```
contacts/          # One markdown file per person
interactions/      # Interaction logs (meetings, calls, messages)
templates/         # Templates for new contacts/interactions
imports/           # Drop zone for CSV/vCard imports
scripts/           # Sync and utility scripts
.schemas/          # YAML schema definitions for validation
```

## Contact Format

Each contact is a markdown file: `contacts/firstname-lastname.md`

YAML frontmatter contains structured, parseable data. The markdown body contains free-form notes, context, and interaction history.

## Design Principles

- **Plain text first**: Everything is markdown + YAML. No database.
- **Git-native**: All history is version controlled.
- **Agent-friendly**: Any AI agent can read/write these files directly.
- **Rust-ready**: YAML frontmatter parses cleanly with `serde_yaml`. Designed for a future Rust CLI/TUI.

## Planned Connectors

- [ ] Apple/iCloud Contacts (CardDAV via `vdirsyncer`)
- [ ] LinkedIn (CSV export import)
- [ ] Outlook/Exchange (Microsoft Graph API)
- [ ] Facebook (data export import)
- [ ] X/Twitter (API or data export)
