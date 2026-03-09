# Contributing to AgenticCRM

Thanks for your interest in contributing to AgenticCRM. This guide covers everything you need to build, test, and submit changes.

## Prerequisites

- **Rust 1.85 or later** -- update with `rustup update stable`
- **Git**
- **macOS recommended** -- keyring integration uses macOS Keychain. On Linux, you'll need `libsecret` installed for credential storage.

## Getting Started

```bash
git clone https://github.com/TheThoughtagen/agenticcrm.git
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

## Releasing

Releases are automated via [release-plz](https://release-plz.dev/) and [cargo-dist](https://opensource.axo.dev/cargo-dist/).

**How it works:**

1. Push to `main` triggers release-plz, which opens a PR with version bump and changelog updates
2. Review and merge the release PR
3. release-plz creates a git tag (e.g., `v0.2.0`)
4. The tag triggers cargo-dist, which:
   - Builds binaries for macOS (ARM64 + x86_64), Linux (x86_64), and Windows (x86_64)
   - Creates a GitHub Release with all artifacts and checksums
   - Generates shell and PowerShell install scripts
   - Publishes the Homebrew formula to the tap

**Manual release (if needed):**

```bash
# Bump version in Cargo.toml, commit, then:
git tag v0.2.0
git push origin v0.2.0
```

**Supported platforms:**

| Platform | Architecture | Install Method |
|----------|-------------|----------------|
| macOS | ARM64 (Apple Silicon) | Homebrew, shell installer |
| macOS | x86_64 (Intel) | Homebrew, shell installer |
| Linux | x86_64 | Shell installer |
| Windows | x86_64 | PowerShell installer |
| Any | Any | `cargo install` from source |
