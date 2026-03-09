# AgenticCRM

A plain-text, agent-friendly personal CRM. Contacts are markdown files with YAML frontmatter, managed through a Rust CLI, interactive TUI, or MCP server.

## Features

- **CLI with 15+ commands** -- add, list, search, show, edit, log, delete, archive, unarchive, due, bulk, bulk-update, sync, serve, tui
- **Interactive TUI** with contact browser, detail view, search, and follow-up dashboard
- **MCP server** for AI agent integration (stdio and Streamable HTTP transports)
- **iCloud/CardDAV bidirectional sync** with conflict detection and selective filtering
- **Bulk operations** with query engine -- field predicates, dry-run preview, JSON pipe support
- **Plain-text storage** -- markdown + YAML frontmatter, fully git-friendly
- **JSON output** for all commands (`-f json`)

## Installation

### Homebrew (macOS/Linux)

```bash
brew install TheThoughtagen/tap/acrm
```

### Shell Installer (macOS/Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/TheThoughtagen/agenticcrm/releases/latest/download/acrm-installer.sh | sh
```

### PowerShell Installer (Windows)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/TheThoughtagen/agenticcrm/releases/latest/download/acrm-installer.ps1 | iex"
```

### From Source

```bash
cargo install --git https://github.com/TheThoughtagen/agenticcrm.git
```

Requires Rust 1.85 or later. Install Rust with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

### Verify

```bash
acrm --help
```

> **Note:** macOS is the primary platform. Linux users need `libsecret` or `gnome-keyring` installed for sync/keyring features.

## Quick Start

```bash
acrm add "Jane Smith"
acrm edit "Jane Smith" --set company="Acme Corp" --set tags='["engineering","vip"]'
acrm log "Jane Smith" -t coffee "Discussed Q3 roadmap"
acrm search acme
acrm due
```

## Usage

### Contact Management

```bash
# Add a new contact
acrm add "Jane Smith"

# List all contacts
acrm list

# Search by name, company, tag, or notes
acrm search acme

# Show full details for a contact
acrm show "Jane Smith"

# Edit contact fields
acrm edit "Jane Smith" --set company="Acme Corp" --set role="CTO"

# Delete a contact
acrm delete "Jane Smith"

# Archive a contact (moves to archive/ directory)
acrm archive "Jane Smith"

# Unarchive a contact (moves back to contacts/)
acrm unarchive "Jane Smith"
```

### Interaction Logging

Log interactions with a type and summary. Supported types: `coffee`, `call`, `email`, `message`, `conference`, `meeting`, `lunch`, `intro`.

```bash
acrm log "Jane Smith" -t coffee "Caught up over coffee at Blue Bottle"
acrm log "Jane Smith" -t email "Sent follow-up on proposal" -n "She said she'd review by Friday"
```

### Follow-up Tracking

```bash
# Show contacts due for follow-up
acrm due
```

### Bulk Operations

Query contacts by field predicates and apply batch operations.

```bash
# Preview dormant contacts
acrm bulk 'status=dormant' --dry-run

# Set status on all matches
acrm bulk 'status=dormant' --set status=active --yes

# Archive by tag
acrm bulk 'tags~old-project' --archive --yes

# Add/remove tags
acrm bulk 'company=Acme Corp' --add-tag partner --yes

# Delete with confirmation
acrm bulk 'status=archived' --delete

# JSON output for piping
acrm search acme -f json | acrm bulk-update --stdin
```

### iCloud Sync

Bidirectional sync with iCloud Contacts via CardDAV.

```bash
# Interactive setup (stores credentials in system keychain)
acrm sync setup

# Full sync (pull then push)
acrm sync

# Pull only
acrm sync pull

# Push local changes to iCloud
acrm sync push

# Dry run -- see what would change
acrm sync --dry-run

# Filter sync by tag or status
acrm sync --tag vip --status active

# Force re-download (ignore ETags)
acrm sync --force
```

### MCP Server

Start an MCP server for AI agent integration.

```bash
# Stdio transport (default, for Claude Desktop / MCP clients)
acrm serve

# Streamable HTTP transport
acrm serve --http --port 3000

# Enable sync operations (disabled by default for safety)
acrm serve --allow-sync
```

See [docs/mcp-setup.md](docs/mcp-setup.md) for a full MCP integration guide.

### Interactive TUI

```bash
acrm tui
```

Browse contacts, view details, search, and manage follow-ups in an interactive terminal interface.

### JSON Output

All commands support JSON output with the `-f json` flag:

```bash
acrm list -f json
acrm show "Jane Smith" -f json
acrm due -f json
```

## Contact Format

Each contact is a markdown file in `contacts/`. YAML frontmatter holds structured data; the markdown body holds free-form notes and interaction history.

Example (`contacts/jane-smith.md`):

```yaml
---
id: "550e8400-e29b-41d4-a716-446655440000"
name: "Jane Smith"
aliases: []
pronouns: "she/her"

# Contact
email: ["jane@acme.com"]
phone: ["+1-555-0100"]
address: []

# Professional
company: "Acme Corp"
role: "CTO"
industry: "Technology"
linkedin: "https://linkedin.com/in/janesmith"

# Social
twitter: ""
github: "janesmith"
website: "https://janesmith.dev"

# Personal
birthday: 1990-06-15
interests: ["rust", "distributed-systems"]

# Relationship
how_we_met: "RustConf 2025"
met_date: 2025-09-14
relationship: friend
tags: ["engineering", "vip"]

# CRM
status: active
follow_up_cadence: "monthly"
last_contacted: 2026-03-01
next_follow_up: 2026-04-01
priority: high

# Source
source: manual
source_id: ""
etag: ""
---

## Notes

Key partner for the infrastructure project.

## Interaction Log

### 2026-03-01 | coffee | Discussed Q3 roadmap

Met at Blue Bottle. She's excited about the new async runtime.

### 2026-01-15 | email | Sent whitepaper on edge computing

She replied with feedback on Section 3.
```

## MCP Integration

`acrm serve` exposes all CRM operations as MCP tools, letting AI agents search, read, create, and update contacts through the standard Model Context Protocol. The server supports both stdio (for Claude Desktop) and Streamable HTTP transports.

Simplest Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "agenticcrm": {
      "command": "acrm",
      "args": ["serve"]
    }
  }
}
```

Available MCP tools: `search_contacts`, `show_contact`, `add_contact`, `edit_contact`, `log_interaction`, `delete_contact`, `archive_contact`, `due_followups`, `sync_contacts`.

See [docs/mcp-setup.md](docs/mcp-setup.md) for the full setup guide, including HTTP transport configuration and sync permissions.

## Configuration

Sync configuration is stored at `~/.config/acrm/sync.toml`. Set it up interactively:

```bash
acrm sync setup
```

## License

MIT -- see [LICENSE](LICENSE)
