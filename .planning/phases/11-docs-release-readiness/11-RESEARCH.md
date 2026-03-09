# Phase 11: Documentation & Release Readiness - Research

**Researched:** 2026-03-09
**Domain:** Project documentation, Cargo packaging, MCP integration guides
**Confidence:** HIGH

## Summary

Phase 11 is a documentation-only phase -- no code changes, only new files and Cargo.toml metadata updates. The project already has a working binary (`acrm`), 167 tests, and all features implemented through Phase 9. The existing README.md is a minimal placeholder from the project's early days and needs a complete rewrite to reflect the CLI tool, MCP server, iCloud sync, bulk operations, and TUI that now exist.

The key deliverables are: (1) a comprehensive README.md with installation, usage examples, and architecture overview; (2) an MCP setup guide showing `claude_desktop_config.json` and Claude Code `.mcp.json` configuration; (3) a LICENSE file; (4) a CONTRIBUTING.md with build/test/contribution instructions; and (5) Cargo.toml metadata fields to enable `cargo install --git`.

**Primary recommendation:** Write documentation that accurately reflects the current feature set, with copy-paste-ready configuration snippets for MCP integration. No code changes needed -- only new/updated files and Cargo.toml metadata.

## Standard Stack

This phase has no library dependencies. It produces documentation files only.

### Cargo.toml Metadata (for `cargo install` support)

| Field | Current | Required Value | Purpose |
|-------|---------|----------------|---------|
| `name` | `"acrm"` | Keep | Binary name for `cargo install` |
| `version` | `"0.1.0"` | Keep or bump | Visible in `--version` |
| `edition` | `"2024"` | Keep | Rust edition |
| `description` | `"Agent-friendly personal CRM — CLI & TUI"` | Keep | Short description |
| `license` | Missing | Add (e.g., `"MIT"`) | Required for good citizenship |
| `repository` | Missing | Add GitHub URL | Enables `cargo install --git` discoverability |
| `readme` | Missing | Add `"README.md"` | Points to readme |
| `homepage` | Missing | Optional | Only if separate from repo |
| `keywords` | Missing | Add `["crm", "contacts", "mcp", "cli"]` | Discoverability |
| `categories` | Missing | Add `["command-line-utilities"]` | Discoverability |

**Note:** `cargo install --git https://github.com/user/agenticcrm.git` works today without any metadata changes -- Cargo just needs a `[[bin]]` target (implicitly provided by `src/main.rs`). The metadata additions above improve discoverability and documentation quality.

## Architecture Patterns

### File Structure for Phase 11 Deliverables

```
/                        # Repository root
├── README.md            # REWRITE - comprehensive project docs
├── LICENSE              # NEW - license file
├── CONTRIBUTING.md      # NEW - build/test/contribute guide
├── docs/
│   └── mcp-setup.md     # NEW - MCP integration guide
├── Cargo.toml           # UPDATE - add metadata fields
└── CLAUDE.md            # EXISTS - agent instructions (no change)
```

### README.md Structure (Recommended)

Following the pattern of well-documented Rust CLI tools (ripgrep, bat, fd):

```markdown
# AgenticCRM

[One-line tagline]

## What is AgenticCRM?
[2-3 paragraph overview: plain-text CRM, markdown contacts, CLI+TUI+MCP]

## Features
[Bulleted feature list with brief descriptions]

## Installation
### From source (cargo install)
### From GitHub release (future)
### Requirements (Rust 1.75+, macOS/Linux)

## Quick Start
[5-command getting started flow: add, list, show, log, search]

## Usage
### Contact Management (add, edit, show, list, search, delete, archive)
### Interaction Logging
### Follow-up Tracking (due)
### Bulk Operations
### iCloud Sync
### MCP Server (link to docs/mcp-setup.md)
### Interactive TUI

## Contact Format
[Show example contact file with frontmatter]

## Configuration
### Sync config (~/.config/acrm/sync.toml)

## MCP Integration
[Brief overview, link to full guide]

## License
```

### MCP Setup Guide Structure

The guide needs to cover two MCP clients:

**Claude Desktop** (`claude_desktop_config.json`):
- Location: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
- Location: `%APPDATA%\Claude\claude_desktop_config.json` (Windows)
- Configuration uses `mcpServers` object with `command` and `args`

**Claude Code** (`.mcp.json` or `claude mcp add`):
- Project-level: `.mcp.json` at repo root
- User-level: `~/.claude.json`
- Can also use `claude mcp add` CLI command

### CONTRIBUTING.md Structure

```markdown
# Contributing to AgenticCRM

## Prerequisites
[Rust toolchain, cargo]

## Building
[cargo build, cargo build --release]

## Testing
[cargo test, how to run specific tests]

## Project Structure
[Brief src/ directory overview]

## Code Style
[rustfmt, clippy, conventions]

## Pull Requests
[Standard PR process]
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| License text | Writing license from memory | SPDX standard text from choosealicense.com | Legal accuracy matters |
| MCP config examples | Guessing config format | Official MCP docs examples adapted | Config format must be exact |
| Cargo.toml metadata | Random fields | The Cargo Book manifest reference | Fields have specific semantics |

**Key insight:** Documentation accuracy is critical. Wrong MCP config snippets will cause user frustration. Every config example should be tested or derived from official sources.

## Common Pitfalls

### Pitfall 1: Stale README Content
**What goes wrong:** README describes features that don't exist or misses features that do
**Why it happens:** Documentation written from memory rather than inspecting actual code
**How to avoid:** Cross-reference every CLI subcommand in `main.rs` and every MCP tool in `mcp/tools.rs` when writing docs
**Warning signs:** Feature list doesn't match `acrm --help` output

### Pitfall 2: Incorrect MCP Configuration Snippets
**What goes wrong:** Users copy-paste config and MCP server fails to connect
**Why it happens:** Wrong binary path, missing args, wrong transport type
**How to avoid:** Provide configurations that assume `cargo install` puts `acrm` on PATH; show both stdio and HTTP examples
**Warning signs:** Config examples reference paths that don't exist after installation

### Pitfall 3: Missing `cargo install` Prerequisites
**What goes wrong:** Users try `cargo install --git` and get build failures
**Why it happens:** Project depends on system libraries or specific Rust edition not documented
**How to avoid:** Document minimum Rust version (edition 2024 requires Rust 1.85+), note any system deps (keyring crate needs macOS Keychain / Linux libsecret)
**Warning signs:** Build fails on fresh system

### Pitfall 4: License Incompatibility
**What goes wrong:** Chosen license conflicts with dependency licenses
**Why it happens:** Not checking dependency license tree
**How to avoid:** All current dependencies (clap, serde, tokio, rmcp, etc.) use MIT/Apache-2.0 dual licensing. MIT or MIT/Apache-2.0 dual license is safe.
**Warning signs:** `cargo deny check licenses` failures

### Pitfall 5: Forgetting to Update .gitignore for New Files
**What goes wrong:** New docs/ directory or files not tracked
**Why it happens:** Overly broad gitignore patterns
**How to avoid:** Current .gitignore has no patterns that would exclude docs/ or markdown files at root -- no issue here

## Code Examples

### MCP Configuration for Claude Desktop (stdio)

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

### MCP Configuration for Claude Desktop (with sync enabled)

```json
{
  "mcpServers": {
    "agenticcrm": {
      "command": "acrm",
      "args": ["serve", "--allow-sync"]
    }
  }
}
```

### MCP Configuration for Claude Code (.mcp.json)

```json
{
  "mcpServers": {
    "agenticcrm": {
      "type": "stdio",
      "command": "acrm",
      "args": ["serve"]
    }
  }
}
```

### MCP HTTP Transport Usage

```bash
# Start HTTP server
acrm serve --http --port 3000

# Connect from any MCP client via Streamable HTTP at http://localhost:3000/mcp
```

### Cargo.toml Metadata Addition

```toml
[package]
name = "acrm"
version = "0.1.0"
edition = "2024"
description = "Agent-friendly personal CRM — CLI & TUI"
license = "MIT"
repository = "https://github.com/USERNAME/agenticcrm"
readme = "README.md"
keywords = ["crm", "contacts", "mcp", "cli", "carddav"]
categories = ["command-line-utilities"]
```

### Installation Command for End Users

```bash
# Install from GitHub
cargo install --git https://github.com/USERNAME/agenticcrm.git

# Verify installation
acrm --help
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| HTTP+SSE MCP transport | Streamable HTTP | MCP spec 2025-03-26 | Docs must reference Streamable HTTP, NOT SSE |
| `vdirsyncer` for CardDAV | Native CardDAV in `acrm sync` | v1.0 | README should NOT mention vdirsyncer |
| Shell scripts only | Full Rust CLI | v1.0 | Old README references scripts; rewrite needed |

**Outdated in current README.md:**
- References `interactions/` directory (doesn't exist -- interactions are in contact files)
- References `scripts/` for sync (replaced by `acrm sync`)
- Lists "Planned Connectors" including Outlook, Facebook, Twitter (out of scope)
- Describes project as "Rust-ready" when it IS a Rust CLI tool now

## Current CLI Command Reference (for README)

Derived from `main.rs` -- all commands the README must document:

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `acrm add <name>` | Add a new contact | |
| `acrm list` | List all contacts | `--tag` |
| `acrm search <query>` | Search contacts | |
| `acrm show <name>` | Show contact details | |
| `acrm edit <name>` | Edit contact fields | `--set key=value` |
| `acrm log <name> -t <type> <summary>` | Log interaction | `--notes` |
| `acrm delete <name>` | Delete contact | `--yes` |
| `acrm archive <name>` | Archive contact | |
| `acrm unarchive <name>` | Unarchive contact | |
| `acrm due` | Show follow-up reminders | |
| `acrm bulk '<query>'` | Bulk operations | `--set`, `--delete`, `--archive`, `--add-tag`, `--remove-tag`, `--dry-run`, `--yes` |
| `acrm bulk-update --stdin` | Bulk update from JSON pipe | Same as bulk |
| `acrm sync` | Bidirectional iCloud sync | `--force`, `--dry-run`, `--tag`, `--status` |
| `acrm sync pull` | Pull from iCloud | Same flags |
| `acrm sync push` | Push to iCloud | Same flags |
| `acrm sync setup` | Configure iCloud credentials | |
| `acrm serve` | Start MCP server | `--http`, `--port`, `--allow-sync` |
| `acrm tui` | Interactive terminal UI | |

All commands support `-f json` for JSON output.

## Open Questions

1. **License choice**
   - What we know: All dependencies are MIT/Apache-2.0 compatible. MIT is simplest and most common for Rust CLIs.
   - What's unclear: User hasn't specified preferred license
   - Recommendation: Default to MIT unless user specifies otherwise

2. **GitHub repository URL**
   - What we know: Cargo.toml needs `repository` field; MCP docs need install URL
   - What's unclear: Exact GitHub URL (username/org)
   - Recommendation: Use placeholder `USERNAME/agenticcrm` in docs, replace when known

3. **Minimum Supported Rust Version (MSRV)**
   - What we know: Uses `edition = "2024"` which requires Rust 1.85+. Current system has Rust 1.92.
   - What's unclear: Whether to pin MSRV in Cargo.toml
   - Recommendation: Document "Rust 1.85 or later" in README. Adding `rust-version = "1.85"` to Cargo.toml is optional but helpful.

4. **System dependencies for keyring crate**
   - What we know: `keyring` crate with `apple-native` feature works on macOS natively. On Linux it needs `libsecret` / `gnome-keyring`.
   - Recommendation: Document macOS as primary supported platform; note Linux requirements for sync features.

## Sources

### Primary (HIGH confidence)
- Cargo manifest format: https://doc.rust-lang.org/cargo/reference/manifest.html
- MCP local server connection: https://modelcontextprotocol.io/docs/develop/connect-local-servers
- Claude Code MCP docs: https://code.claude.com/docs/en/mcp
- Claude Desktop MCP setup: https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop

### Secondary (MEDIUM confidence)
- Source code inspection of `main.rs`, `Cargo.toml`, `mcp/mod.rs` for feature inventory

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no libraries needed, just file creation and metadata
- Architecture: HIGH - well-established patterns for Rust CLI project documentation
- Pitfalls: HIGH - based on direct code inspection and known Cargo/MCP patterns

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (documentation patterns are stable)
