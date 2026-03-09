# Architecture Research

**Domain:** MCP server, bulk operations, and LinkedIn automation for existing Rust CLI CRM
**Researched:** 2026-03-08
**Confidence:** HIGH (existing codebase well-understood, MCP SDK is official and documented)

## System Overview

```
                        Entry Points
  ┌──────────┐    ┌──────────────┐    ┌───────────────┐
  │  CLI      │    │  MCP Server  │    │  LinkedIn     │
  │  (clap)   │    │  (rmcp)      │    │  Automation   │
  └─────┬─────┘    └──────┬───────┘    └───────┬───────┘
        │                 │                    │
        │    ┌────────────┴────────┐           │
        │    │  MCP Tool Handlers  │           │
        │    │  (mcp/tools.rs)     │           │
        │    └────────────┬────────┘           │
        │                 │                    │
  ┌─────┴─────────────────┴────────────────────┴──────┐
  │                  Operations Layer                   │
  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
  │  │ commands/ │  │  query/  │  │  linkedin/       │  │
  │  │ (existing)│  │  (NEW)   │  │  (NEW)           │  │
  │  └─────┬────┘  └────┬─────┘  └────────┬─────────┘  │
  │        │            │                  │            │
  ├────────┴────────────┴──────────────────┴────────────┤
  │                   Core Layer                        │
  │  ┌──────────┐  ┌─────────────┐  ┌──────────────┐   │
  │  │ store.rs │  │frontmatter.rs│  │validation.rs │   │
  │  │          │  │             │  │              │   │
  │  └──────────┘  └─────────────┘  └──────────────┘   │
  ├─────────────────────────────────────────────────────┤
  │                   Data Layer                        │
  │  ┌────────────────────────────────────────────┐     │
  │  │           contacts/*.md (flat files)        │     │
  │  └────────────────────────────────────────────┘     │
  └─────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Status |
|-----------|---------------|--------|
| `store.rs` | File I/O, `ContactFile` parsing/writing, CRM root resolution | EXISTS -- no changes needed |
| `frontmatter.rs` | Raw YAML preservation, field updates, array updates | EXISTS -- no changes needed |
| `validation.rs` | Contact field validation | EXISTS -- no changes needed |
| `commands/` | Individual CLI command handlers (add, edit, search, etc.) | EXISTS -- refactor to delegate to ops.rs |
| `models/contact.rs` | `Contact` struct, `ContactFile`, enums | EXISTS -- no changes needed |
| `format.rs` | Human/JSON output formatting | EXISTS -- no changes needed |
| `ops.rs` | Pure business logic extracted from commands/ | **NEW** -- prerequisite for MCP |
| `mcp/` | MCP server setup, tool definitions, transport | **NEW** |
| `query/` | Query parser + filter engine for bulk operations | **NEW** |
| `linkedin/` | CSV import logic (Rust port) + Playwright automation | **NEW** |

## Recommended Project Structure

```
src/
├── commands/           # Existing CLI command handlers (thin wrappers)
│   ├── mod.rs          # Add pub mod bulk; pub mod import;
│   ├── add.rs          # Delegates to ops::add_contact()
│   ├── edit.rs         # Delegates to ops::edit_contact()
│   ├── search.rs       # Delegates to ops::search_contacts()
│   ├── show.rs         # Delegates to ops::show_contact()
│   ├── log.rs          # Delegates to ops::log_interaction()
│   ├── due.rs          # Delegates to ops::due_contacts()
│   ├── list.rs         # Delegates to ops::list_contacts()
│   ├── delete.rs       # Delegates to ops::delete_contact()
│   ├── archive.rs      # Delegates to ops::archive_contact()
│   ├── sync.rs         # Existing CardDAV sync (unchanged)
│   ├── bulk.rs         # NEW -- acrm bulk subcommand
│   └── import.rs       # NEW -- acrm import linkedin subcommand
├── models/             # Existing data models (unchanged)
│   ├── mod.rs
│   └── contact.rs
├── sync/               # Existing CardDAV sync (unchanged)
├── tui/                # Existing TUI (unchanged)
├── mcp/                # NEW -- MCP server module
│   ├── mod.rs          # Server init, transport setup
│   ├── server.rs       # ServerHandler impl with #[tool(tool_box)]
│   └── tools.rs        # Individual #[tool] definitions
├── query/              # NEW -- Query engine for bulk ops
│   ├── mod.rs
│   ├── parser.rs       # Query syntax parser (key=value AND ...)
│   └── filter.rs       # Filter execution on Vec<ContactFile>
├── linkedin/           # NEW -- LinkedIn automation
│   ├── mod.rs
│   ├── csv_import.rs   # Rust port of import-linkedin.sh with dedup
│   └── automation.rs   # Shell-out to Playwright script for CSV export
├── ops.rs              # NEW -- Pure business logic (core of the refactor)
├── format.rs           # Existing (unchanged)
├── frontmatter.rs      # Existing (unchanged)
├── store.rs            # Existing (unchanged)
├── validation.rs       # Existing (unchanged)
└── main.rs             # Add Serve, Bulk, Import subcommands
scripts/
└── linkedin-export.js  # NEW -- Playwright script for LinkedIn CSV export
```

### Structure Rationale

- **`ops.rs` (NEW critical path):** Currently business logic lives inside command handlers that own `println!` and format output. The MCP server needs the same logic without stdout. Extract pure operations (search, filter, edit, add, log) returning `Result<T>` -- both commands and MCP tools call into this shared layer. This is the single most important architectural change.
- **`mcp/`:** Isolated module because the MCP server has its own lifecycle (long-running async process) vs the rest of the codebase (synchronous, run-and-exit). Contains the rmcp `ServerHandler` impl and tool definitions.
- **`query/`:** Separated from commands because the query engine serves both `acrm bulk` CLI and MCP tool filtering. The parser converts `"status=dormant AND tag=linkedin-import"` into filter predicates; the filter applies them to `Vec<ContactFile>`.
- **`linkedin/`:** Isolated because Playwright automation is experimental and has heavy external dependencies. CSV import is clean Rust; automation shells out to a JS script.

## Architectural Patterns

### Pattern 1: Operations Layer Extraction

**What:** Extract business logic from CLI command handlers into pure functions in `ops.rs` that return structured results, then have both CLI commands and MCP tools call these functions.
**When to use:** Now -- prerequisite for MCP integration.
**Trade-offs:** Small refactoring effort up front, but eliminates code duplication between CLI and MCP. Every existing command handler becomes a thin wrapper.

**Example:**
```rust
// ops.rs -- pure business logic, no I/O formatting
pub fn search_contacts(root: &Path, query: &str) -> Result<Vec<ContactFile>> {
    let contacts = store::load_all_contacts(root)?;
    let query_lower = query.to_lowercase();
    Ok(contacts.into_iter().filter(|cf| {
        let c = &cf.contact;
        c.name.to_lowercase().contains(&query_lower)
            || c.company.to_lowercase().contains(&query_lower)
            || c.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            || cf.body.to_lowercase().contains(&query_lower)
    }).collect())
}

pub fn add_contact(root: &Path, name: &str) -> Result<ContactFile> { ... }
pub fn edit_contact(root: &Path, name: &str, sets: &[String]) -> Result<EditResult> { ... }
pub fn log_interaction(root: &Path, name: &str, itype: &str, summary: &str, notes: Option<&str>) -> Result<LogResult> { ... }
pub fn due_contacts(root: &Path) -> Result<Vec<DueContact>> { ... }
pub fn show_contact(root: &Path, name: &str) -> Result<ContactFile> { ... }

// commands/search.rs -- becomes a thin wrapper
pub fn run(query: &str, fmt: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let results = ops::search_contacts(&root, query)?;
    let search_results: Vec<SearchResult> = results.iter().map(|cf| SearchResult { ... }).collect();
    format::output_list(&search_results, fmt, "match(es)")
}
```

### Pattern 2: Async Boundary at MCP Layer Only

**What:** Keep the entire core codebase synchronous. The MCP server uses `tokio::task::spawn_blocking` to call into sync operations from the async MCP handler.
**When to use:** This project. The codebase is 4,700+ LOC of synchronous Rust. Converting to async would be a rewrite with no benefit for file I/O.
**Trade-offs:** Slight overhead from `spawn_blocking` (thread pool), but the alternative (async rewrite) is catastrophic.

**Example:**
```rust
// mcp/server.rs
#[tool(description = "Search contacts by name, company, tag, or notes")]
async fn search(
    &self,
    #[tool(param, description = "Search query string")] query: String,
) -> Result<CallToolResult, McpError> {
    let result = tokio::task::spawn_blocking(move || {
        let root = store::find_crm_root()?;
        ops::search_contacts(&root, &query)
    }).await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json = serde_json::to_string_pretty(
        &result.iter().map(|cf| &cf.contact).collect::<Vec<_>>()
    ).unwrap();
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
```

### Pattern 3: Simple Query DSL as Filter Predicates

**What:** Parse a simple query string into filter predicates applied to `Vec<ContactFile>` in memory. No database, no query planner.
**When to use:** Bulk operations and MCP filtered queries.
**Trade-offs:** Limited to what fits in memory (fine for <10K contacts). Simple to implement and debug.

**Supported syntax:**
```
status=dormant                              # exact match
status=dormant AND tag=linkedin-import      # conjunction
name~smith                                  # contains
last_contacted<2025-01-01                   # date comparison
next_follow_up>2026-03-01                   # date comparison
company="Acme Corp"                         # quoted value with spaces
```

**Example:**
```rust
// query/parser.rs
pub enum Op { Eq, Contains, Lt, Gt }

pub struct Predicate {
    pub field: String,
    pub op: Op,
    pub value: String,
}

pub fn parse_query(input: &str) -> Result<Vec<Predicate>> {
    input.split(" AND ")
        .map(|part| parse_predicate(part.trim()))
        .collect()
}

// query/filter.rs
pub fn apply_predicates(contacts: Vec<ContactFile>, preds: &[Predicate]) -> Vec<ContactFile> {
    contacts.into_iter()
        .filter(|cf| preds.iter().all(|p| matches_predicate(cf, p)))
        .collect()
}
```

## Data Flow

### MCP Tool Call Flow

```
AI Agent (Claude, Cursor, etc.)
    |
    v (JSON-RPC over stdio)
rmcp transport layer (tokio async)
    |
    v (deserialize tool call, dispatch)
mcp/server.rs -- #[tool] handler
    |
    v (spawn_blocking -- cross async/sync boundary)
ops.rs -- pure business logic (synchronous)
    |
    v (read/write)
store.rs --> contacts/*.md
    |
    v (Result<T>)
mcp/server.rs -- serialize to JSON Content
    |
    v (JSON-RPC response)
AI Agent
```

### Bulk Operation Flow

```
acrm bulk 'status=dormant AND last_contacted<2025-01-01' --set status=archived
    |
    v
commands/bulk.rs -- parse args
    |
    v
query/parser.rs --> Vec<Predicate>
    |
    v
store::load_all_contacts() + query/filter.rs --> matched Vec<ContactFile>
    |
    v (for each matched contact)
ops::edit_contact() --> write updated file
    |
    v
format: human summary or JSON array of results
```

### JSON Pipe Flow

```
acrm search "linkedin" --format json | acrm bulk --stdin --set status=dormant
    |                                        |
    v                                        v
stdout: JSON array of contacts         stdin: parse JSON, extract contact names/paths
                                              |
                                              v
                                        For each: ops::edit_contact() --> write
```

The `--stdin` flag reads JSON array from stdin. Each object must have a `name` or `path` field to identify the contact. This avoids re-loading and re-filtering -- the upstream command already selected the contacts.

### LinkedIn Automation Flow

```
acrm import linkedin <path/to/Connections.csv>     # manual CSV import
acrm import linkedin --auto                        # automated export + import

Manual path:
    Connections.csv --> linkedin/csv_import.rs --> parse rows
        --> linkedin/dedup.rs --> compare against existing (source=linkedin)
        --> ops::add_contact() for new / ops::edit_contact() for changed

Auto path:
    linkedin/automation.rs --> shell out to scripts/linkedin-export.js
        --> Playwright: login, navigate, request export, poll for download
        --> CSV downloaded to temp dir
        --> continue with manual path above
```

## Key Integration Points

### What Changes in Existing Code

| File | Change | Scope |
|------|--------|-------|
| `main.rs` | Add `Serve`, `Bulk`, `Import` subcommands to `Commands` enum | Small -- 3 new match arms |
| `commands/mod.rs` | Add `pub mod bulk; pub mod import;` | Trivial |
| `Cargo.toml` | Add rmcp, tokio, csv dependencies | Dependencies only |
| `commands/search.rs` | Extract filter logic to `ops.rs`, call `ops::search_contacts()` | Refactor -- behavior unchanged |
| `commands/edit.rs` | Extract edit logic to `ops.rs`, call `ops::edit_contact()` | Refactor -- behavior unchanged |
| `commands/add.rs` | Extract add logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/log.rs` | Extract log logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/due.rs` | Extract due logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/show.rs` | Extract show logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/list.rs` | Extract list logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/delete.rs` | Extract delete logic to `ops.rs` | Refactor -- behavior unchanged |
| `commands/archive.rs` | Extract archive logic to `ops.rs` | Refactor -- behavior unchanged |

### What Does NOT Change

- `store.rs` -- Already provides the right abstractions
- `frontmatter.rs` -- Used transitively, no direct changes
- `validation.rs` -- Called by `store::write_contact()`
- `models/` -- `Contact` and `ContactFile` already `Serialize`
- `sync/` -- CardDAV sync is independent
- `tui/` -- TUI is independent
- `format.rs` -- CLI formatting stays as-is

### New Dependencies

| Crate | Version | Purpose | Feature Flags |
|-------|---------|---------|---------------|
| `rmcp` | 0.16 | Official MCP SDK | `server`, `transport-io`, `macros` |
| `tokio` | 1 | Async runtime (MCP server only) | `full` |
| `csv` | 1 | LinkedIn CSV parsing | default |

Note: `serde_json` and `uuid` already present in dependencies.

## Binary Strategy

**Recommended: Same binary, new subcommand.**

```
acrm serve                      # Start MCP server (stdio transport)
acrm serve --transport http     # Future: Streamable HTTP transport
acrm bulk '<query>' --set k=v   # Bulk operations
acrm import linkedin <file>     # LinkedIn CSV import
acrm import linkedin --auto     # LinkedIn automated export + import
```

Rationale: Single binary eliminates distribution complexity. The tokio dependency adds ~2MB but only the `serve` subcommand uses the async runtime. All other commands remain synchronous -- tokio is only initialized when `acrm serve` is called.

## MCP Transport Choice

**Use stdio transport. Do NOT implement Streamable HTTP initially.**

- stdio is standard for local MCP tools (Claude Desktop, Cursor, etc. spawn the binary as a child process)
- SSE is deprecated in the MCP spec -- replaced by Streamable HTTP
- Streamable HTTP is for remote/multi-user servers -- this is a personal local-first tool
- stdio requires zero network configuration, zero auth setup

The `--transport http` flag can be added later if remote access is needed, but stdio covers the primary use case.

## MCP Tools to Expose

| Tool Name | Maps to | Read/Write | Parameters |
|-----------|---------|------------|------------|
| `search_contacts` | `ops::search_contacts()` | Read | `query: String` |
| `show_contact` | `ops::show_contact()` | Read | `name: String` |
| `list_contacts` | `ops::list_contacts()` | Read | `tag: Option<String>` |
| `due_followups` | `ops::due_contacts()` | Read | (none) |
| `add_contact` | `ops::add_contact()` | Write | `name: String` |
| `edit_contact` | `ops::edit_contact()` | Write | `name: String, fields: Vec<KeyValue>` |
| `log_interaction` | `ops::log_interaction()` | Write | `name, type, summary, notes` |
| `delete_contact` | `ops::delete_contact()` | Write | `name: String` |
| `archive_contact` | `ops::archive_contact()` | Write | `name: String` |
| `bulk_query` | `query + ops` | Read | `query: String` |
| `bulk_update` | `query + ops::edit` | Write | `query: String, fields: Vec<KeyValue>` |

## Anti-Patterns

### Anti-Pattern 1: Duplicating Business Logic in MCP Handlers

**What people do:** Copy-paste logic from CLI command handlers into MCP tool handlers.
**Why it's wrong:** Two copies diverge over time. Bug fixes applied to one path but not the other.
**Do this instead:** Extract to `ops.rs`. Both CLI commands and MCP tools become thin wrappers.

### Anti-Pattern 2: Making the Entire Codebase Async

**What people do:** See that rmcp requires async, convert everything to async/await.
**Why it's wrong:** This is a file I/O tool on local disk. Async adds complexity (lifetimes, `Send` bounds, colored function problem) with zero performance benefit. 4,700+ LOC of working synchronous code would need rewriting.
**Do this instead:** Keep core synchronous. `spawn_blocking` at the MCP boundary only.

### Anti-Pattern 3: Building a Complex Query Language Parser

**What people do:** Build a recursive-descent parser for SQL-like queries with OR, grouping, subqueries.
**Why it's wrong:** Over-engineered for <10K contacts filtered in memory. Parsing edge cases consume weeks.
**Do this instead:** Simple `key{op}value` predicates joined by ` AND `. For complex queries, pipe JSON through `jq`.

### Anti-Pattern 4: Running Playwright from Rust Directly

**What people do:** Embed Playwright via `playwright-rust` crate FFI.
**Why it's wrong:** `playwright-rust` still requires Node.js and npm Playwright. Adds massive dependency for an experimental feature. Breaks the "no runtime dependencies" constraint.
**Do this instead:** Ship a small JS Playwright script as `scripts/linkedin-export.js`. Shell out to it via `std::process::Command`. Independently testable, independently updatable.

### Anti-Pattern 5: MCP Server as a Separate Binary

**What people do:** Create a separate `acrm-mcp` binary to avoid pulling tokio into the main binary.
**Why it's wrong:** Two binaries to build, distribute, and keep in sync. Requires extracting shared code into a library crate -- significant restructuring.
**Do this instead:** Single binary with `acrm serve` subcommand. Tokio only initializes when `serve` is called. ~2MB binary size increase is acceptable.

## Build Order (Dependency-Aware)

```
Phase 1: Operations Layer Extraction (PREREQUISITE)
    Extract ops.rs from commands/
    Refactor all commands to delegate to ops.rs
    Verify all existing tests pass
    |
Phase 2: Bulk Operations
    query/parser.rs -- parse query syntax
    query/filter.rs -- apply predicates to contacts
    commands/bulk.rs -- CLI subcommand
    JSON stdin pipe support (--stdin flag)
    |
Phase 3: MCP Server
    Add rmcp + tokio dependencies
    mcp/server.rs -- ServerHandler with #[tool(tool_box)]
    mcp/tools.rs -- tool definitions calling ops.rs
    commands/serve.rs -- CLI entry point
    |
Phase 4: LinkedIn Automation (independent, experimental)
    linkedin/csv_import.rs -- Rust port of import-linkedin.sh
    linkedin/dedup.rs -- change detection vs existing contacts
    scripts/linkedin-export.js -- Playwright automation script
    linkedin/automation.rs -- shell out + import orchestration
    commands/import.rs -- CLI subcommand
```

**Phase ordering rationale:**
1. **ops.rs first** -- both MCP and bulk ops need it. Pure refactor, no new features. Existing tests validate behavior preservation.
2. **Bulk ops before MCP** -- simpler (no async, no protocol), validates ops layer works for multi-contact operations. Query engine also reusable by MCP.
3. **MCP after bulk** -- depends on ops.rs being stable. Introduces async boundary + new protocol. Benefits from query engine already existing.
4. **LinkedIn last** -- marked experimental, external dependencies (Node.js + Playwright), fully independent of other features. CSV import (pure Rust) can ship without Playwright automation.

## Sources

- [Official Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk) -- v0.16.0, official implementation with `#[tool]` macros
- [rmcp on crates.io](https://crates.io/crates/rmcp) -- v0.16.0 with server, transport-io, macros features
- [MCP Transports Specification](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports) -- stdio vs Streamable HTTP decision
- [Why MCP Deprecated SSE](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/) -- SSE replaced by Streamable HTTP
- [MCP Transport Comparison](https://mcpcat.io/guides/comparing-stdio-sse-streamablehttp/) -- stdio for local tools, HTTP for remote
- [LinkedIn Export Help](https://www.linkedin.com/help/linkedin/answer/a566336/export-connections-from-linkedin) -- CSV format and limitations
- [Playwright Rust Bindings](https://github.com/octaltree/playwright-rust) -- requires Node.js, not standalone Rust
- Existing codebase analysis: all source files in `src/` (HIGH confidence)

---
*Architecture research for: AgenticCRM v1.2 MCP, Bulk Ops & LinkedIn*
*Researched: 2026-03-08*
