# Technology Stack

**Project:** AgenticCRM - Milestone 2 (CardDAV sync, TUI, MCP server, JSON output)
**Researched:** 2026-03-05
**Note:** WebSearch/WebFetch/Bash tools were unavailable during research. Versions are based on training data (May 2025). All versions marked MEDIUM confidence -- verify with `cargo search` or crates.io before adding to Cargo.toml.

## Existing Stack (Do Not Change)

Already in Cargo.toml -- these stay as-is:

| Technology | Version | Purpose |
|------------|---------|---------|
| clap | 4 (derive) | CLI argument parsing |
| serde | 1 (derive) | Serialization framework |
| serde_yaml | 0.9 | YAML frontmatter parsing |
| uuid | 1 (v4) | Contact ID generation |
| chrono | 0.4 (serde) | Date handling |
| colored | 3 | Terminal colors (CLI mode) |
| dirs | 6 | Home directory resolution |
| walkdir | 2 | Directory traversal |
| anyhow | 1 | Error handling |

## New Dependencies

### JSON Output (Simplest -- Do First)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| serde_json | 1 | JSON serialization for `--json` output flag | De facto standard. Already using serde derives on Contact, so JSON output is nearly free. | HIGH |

**Rationale:** The Contact struct already derives `Serialize`. Adding `serde_json` and a `--json` flag to clap is a one-afternoon task. No architecture changes needed.

**Implementation approach:** Add a global `--json` / `--output json` flag to the top-level `Cli` struct. Each command checks the flag and either prints colored text (existing) or `serde_json::to_string_pretty()`.

### TUI (ratatui)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| ratatui | ~0.29 | Terminal UI framework | The Rust TUI framework. Fork of tui-rs, actively maintained, massive ecosystem. No serious alternative exists. | HIGH (library), MEDIUM (version) |
| crossterm | ~0.28 | Terminal backend for ratatui | Default backend for ratatui on all platforms. Works on macOS, Linux, Windows. | HIGH (library), MEDIUM (version) |
| tui-input | ~0.11 | Text input widget for ratatui | Handles text input fields (search, editing). Saves writing input handling from scratch. | MEDIUM |

**Why ratatui, not alternatives:**
- `cursive` -- Older, less active, callback-heavy API, smaller widget ecosystem
- `tui-rs` -- Unmaintained, ratatui IS its successor
- `termion` -- Too low-level, ratatui uses crossterm as backend anyway
- `egui` -- GUI framework, not terminal-based

**Architecture note:** ratatui uses an immediate-mode rendering pattern. You'll need a main event loop (`crossterm::event::read()`) and an `App` struct holding UI state. This is a separate binary entry point or a new subcommand (`acrm tui`), NOT integrated into the existing command dispatch.

### CardDAV / vCard Sync

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| reqwest | ~0.12 | HTTP client for CardDAV | Most popular async HTTP client in Rust. Needed for PROPFIND/REPORT/PUT/DELETE against CardDAV servers. | HIGH (library), MEDIUM (version) |
| tokio | 1 (features: full) | Async runtime | Required by reqwest. Also needed for MCP server. Use `full` features. | HIGH |
| quick-xml | ~0.37 | XML parsing for CardDAV/WebDAV | CardDAV uses XML for PROPFIND/REPORT requests and responses. quick-xml is the fastest, most popular XML parser in Rust. | HIGH (library), MEDIUM (version) |
| base64 | ~0.22 | Basic auth encoding | iCloud CardDAV uses HTTP Basic Auth (username + app-specific password). | HIGH (library), MEDIUM (version) |
| keyring | ~3 | OS keychain access | Store iCloud app-specific passwords in macOS Keychain, not plaintext config. | MEDIUM |

**Why NOT use an existing CardDAV library:**
- There is no mature, maintained Rust CardDAV client library. The Rust ecosystem has fragmented, incomplete CardDAV crates (e.g., `webdav-rs` is unmaintained, `dav-server` is server-side only).
- CardDAV is WebDAV + vCard. The protocol surface for a CRM sync is small: PROPFIND to list contacts, GET to fetch vCards, PUT to create/update, DELETE to remove. This is ~200 lines of HTTP+XML on top of reqwest.
- Building a thin CardDAV client layer is the right call for this project.

**vCard parsing:**

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| vcard | ~0.9 (or similar) | Parse/generate vCard 3.0/4.0 format | iCloud uses vCard 3.0. Need to parse vCards from CardDAV into Contact structs and serialize Contact back to vCard. | LOW -- verify crate exists and is maintained |

**Alternative for vCard:** If no good `vcard` crate exists, use `ical` crate (which handles vCard as well since vCard shares the iCalendar property format) or write a minimal vCard parser. vCard 3.0 is a simple text format (similar to INI), not complex to parse manually for the subset of properties we need (FN, N, EMAIL, TEL, ADR, ORG, TITLE, BDAY, URL, NOTE, UID).

**iCloud-specific notes:**
- Endpoint: `https://contacts.icloud.com`
- Auth: Apple ID + app-specific password (generated at appleid.apple.com)
- Discovery: `/.well-known/carddav` -> PROPFIND for principal URL -> address book home set -> address book URL
- Sync: Use `getctag` and `getetag` for change detection (avoid full re-download)
- CTag changes when any card changes; ETags are per-card

### MCP Server

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| mcp-server (or rmcp) | ~0.1-0.2 | Model Context Protocol server SDK | Anthropic-backed protocol for AI tool integration. Need to verify which Rust crate is canonical. | LOW |
| serde_json | 1 | JSON-RPC message serialization | MCP uses JSON-RPC 2.0 over stdio. Already adding serde_json for CLI output. | HIGH |
| tokio | 1 | Async runtime | MCP server needs async I/O for stdio transport. Already adding for CardDAV. | HIGH |

**MCP ecosystem assessment:**

The MCP Rust SDK situation (as of my training data, May 2025):
- The official MCP SDKs from Anthropic exist for TypeScript and Python
- Rust community crates exist but are early-stage. Look for `mcp-server`, `rmcp`, `mcp-rs`, or `modelcontextprotocol` on crates.io
- **Fallback approach:** MCP over stdio is JSON-RPC 2.0 -- a well-defined, simple protocol. If no good Rust SDK exists, implement the JSON-RPC layer directly using `serde_json` + `tokio::io`. The MCP spec defines ~10 methods total; a stdio transport is straightforward.

**IMPORTANT: Verify before committing to a crate.** Run `cargo search mcp` and check crates.io/GitHub for the canonical Rust MCP implementation. This is the highest-risk dependency in the stack.

**MCP tools to expose:**
- `list_contacts` -- search/filter contacts, return JSON
- `get_contact` -- get full contact details
- `add_contact` -- create new contact
- `log_interaction` -- log interaction to a contact
- `get_due_followups` -- list contacts due for follow-up

### Supporting / Cross-Cutting

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tokio | 1 (full) | Async runtime | Shared by reqwest (CardDAV), MCP server. Single async runtime for the whole app. | HIGH |
| tracing | ~0.1 | Structured logging | Replace ad-hoc `eprintln!` with proper logging. Essential for debugging sync and MCP server issues. Filter with `RUST_LOG`. | HIGH (library), MEDIUM (version) |
| tracing-subscriber | ~0.3 | Log output formatting | Companion to tracing. Provides stdout/stderr log formatting. | HIGH (library), MEDIUM (version) |
| toml | ~0.8 | Config file parsing | Need a config file for iCloud credentials path, sync settings, MCP server settings. TOML is the Rust ecosystem standard. | MEDIUM |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| TUI framework | ratatui | cursive | Less active, callback-heavy API, smaller widget set |
| TUI backend | crossterm | termion | termion is Unix-only; crossterm is cross-platform and ratatui's default |
| HTTP client | reqwest | hyper | hyper is too low-level; reqwest wraps it with a usable API |
| HTTP client | reqwest | ureq | ureq is blocking-only; we need async for MCP server anyway |
| XML parser | quick-xml | xml-rs | xml-rs is slower and less ergonomic; quick-xml is the ecosystem standard |
| Async runtime | tokio | async-std | tokio has won the async runtime war; reqwest requires it |
| Config format | TOML | YAML | Already using YAML for contacts; TOML for config prevents confusion and is Rust-idiomatic |
| Logging | tracing | log + env_logger | tracing is the modern standard, supports structured fields, spans for sync operations |
| Keychain | keyring | manual file | Never store passwords in plaintext config files |

## What NOT to Use

| Technology | Why Avoid |
|------------|-----------|
| `vdirsyncer` (Python) | PROJECT.md mentions this but constraint is "no runtime dependencies beyond compiled binary". Shell out to Python = breaks constraint. |
| `diesel` / `sqlx` / any ORM | No database. Flat file architecture is a core design decision. |
| `actix-web` / `axum` | MCP server uses stdio transport, not HTTP. No web server needed. |
| `tui-rs` | Unmaintained predecessor to ratatui. |
| `openssl` | Prefer `rustls` (via reqwest's `rustls-tls` feature) for TLS. No system OpenSSL dependency. |
| `rusqlite` | Even for sync state/caching -- use a JSON file instead. Keep the "no database" constraint. |

## Cargo.toml Additions

```toml
# JSON output (Phase 1 -- trivial)
serde_json = "1"

# TUI (Phase 2)
ratatui = "0.29"      # VERIFY version
crossterm = "0.28"    # VERIFY version

# CardDAV sync (Phase 3)
reqwest = { version = "0.12", features = ["rustls-tls", "json"] }  # VERIFY version
tokio = { version = "1", features = ["full"] }
quick-xml = "0.37"    # VERIFY version
base64 = "0.22"       # VERIFY version
keyring = "3"         # VERIFY version

# MCP server (Phase 4)
# TBD -- verify crate on crates.io first
# Fallback: implement JSON-RPC 2.0 over stdio with serde_json + tokio

# Cross-cutting
tracing = "0.1"
tracing-subscriber = "0.3"
toml = "0.8"          # VERIFY version
```

**IMPORTANT:** Every version marked "VERIFY" should be confirmed with `cargo search <crate>` or crates.io before adding to Cargo.toml. Versions are from May 2025 training data and may be outdated by 10 months.

## Feature Flags Strategy

Use Cargo features to keep the binary modular:

```toml
[features]
default = ["cli"]
cli = []
tui = ["ratatui", "crossterm"]
sync = ["reqwest", "tokio", "quick-xml", "base64", "keyring"]
mcp = ["tokio", "serde_json"]  # plus MCP SDK if available
```

This lets users compile just the CLI without pulling in TUI/sync/MCP dependencies. Important for fast compile times during development.

## Sources

- Training data knowledge (May 2025) -- all recommendations MEDIUM confidence unless noted
- Existing codebase analysis (Cargo.toml, src/)
- PROJECT.md constraints and decisions
- CardDAV protocol: RFC 6352, RFC 4918 (WebDAV), RFC 6350 (vCard 4.0), RFC 2426 (vCard 3.0)
- MCP specification: https://modelcontextprotocol.io

---

*Stack research: 2026-03-05*
*Confidence caveat: WebSearch/WebFetch were unavailable. Verify all version numbers before use.*
