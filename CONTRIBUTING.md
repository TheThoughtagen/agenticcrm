# Contributing to AgenticCRM

Thanks for your interest in contributing to AgenticCRM. This guide covers everything you need to build, test, and submit changes.

## Prerequisites

- **Rust 1.85 or later** -- update with `rustup update stable`
- **Git**
- **macOS recommended** -- keyring integration uses macOS Keychain. On Linux, you'll need `libsecret` installed for credential storage.

## Getting Started

```bash
git clone https://github.com/pmannion/agenticcrm.git
cd agenticcrm
cargo build
cargo test
```

If the build succeeds and all tests pass, you're ready to go.

## Project Structure

```
src/
  main.rs          -- CLI entry point and command definitions (clap derive)
  commands/        -- CLI command handlers (thin wrappers around ops)
  ops/             -- Business logic layer (used by CLI, TUI, and MCP)
  mcp/             -- MCP server implementation (tools, resources, transport)
  tui/             -- Interactive terminal UI (ratatui)
  sync/            -- CardDAV sync engine
  models/          -- Contact data model, parsing, serialization
  store.rs         -- Contact file I/O
  query.rs         -- Query engine for bulk operations
  validation.rs    -- Schema validation
  format.rs        -- Output formatting
  frontmatter.rs   -- YAML frontmatter parsing

contacts/          -- Contact markdown files (data directory)
.schemas/          -- YAML schema definitions
templates/         -- Contact templates
scripts/           -- Helper scripts (import, add contact)
docs/              -- Documentation (MCP setup, etc.)
```

**Key architecture principle:** All business logic lives in `src/ops/`. CLI commands, TUI handlers, and MCP tools are thin wrappers that delegate to ops functions. This keeps behavior consistent across all interfaces.

## Development Workflow

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy

# Format code
cargo fmt

# Release build
cargo build --release

# Run the CLI
cargo run -- --help
```

## Code Conventions

- **Business logic in `src/ops/`** -- CLI commands are thin wrappers, never put logic directly in command handlers
- **Error handling** -- Functions return `Result<T>` with `OpsError` (thiserror-based). Variants: `NotFound`, `AmbiguousMatch`, `ValidationFailed`, `Io`, `Internal`, `SyncError`
- **Dates** -- Always `YYYY-MM-DD` format
- **Tags** -- Lowercase, hyphenated (e.g., `open-source`, `ai-ml`)
- **Contact filenames** -- Lowercase, hyphen-separated: `firstname-lastname.md`
- **Empty fields** -- Use `""` for strings, `[]` for arrays, leave blank for dates

## Testing

- **Unit tests** live alongside source code in each module
- **Integration tests** use the `tempfile` crate for isolated test directories
- **Run a specific test:** `cargo test test_name`
- **Run with output:** `cargo test -- --nocapture`
- **Run tests for a module:** `cargo test ops::contact`

When adding new functionality, include tests that cover both the success path and error cases.

## Pull Requests

- Keep PRs focused on a single concern
- Ensure `cargo test` passes before submitting
- Ensure `cargo clippy` produces no warnings
- Run `cargo fmt` to maintain consistent formatting
- Describe **what** changed and **why** in the PR description
