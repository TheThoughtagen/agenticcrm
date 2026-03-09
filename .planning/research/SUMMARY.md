# Project Research Summary

**Project:** AgenticCRM v1.2 -- MCP Server, Bulk Ops & LinkedIn Automation
**Domain:** Rust CLI personal CRM with flat-file markdown storage
**Researched:** 2026-03-08
**Confidence:** HIGH

## Executive Summary

AgenticCRM v1.2 adds three capabilities to an existing, stable Rust CLI CRM (4,700+ LOC): an MCP server for AI agent integration, bulk query/edit operations for managing contacts at scale, and LinkedIn CSV import with optional browser automation. The existing codebase is entirely synchronous and well-structured around a flat-file markdown store. The core architectural challenge is introducing an async MCP server layer without contaminating the synchronous codebase -- solved cleanly by extracting shared business logic into an `ops.rs` operations layer and using `tokio::task::spawn_blocking` at the MCP boundary.

The recommended approach is a four-phase build: (1) extract operations layer from CLI command handlers (pure refactor, zero behavior change), (2) build bulk operations with a simple query DSL, (3) add MCP server via the official rmcp SDK with stdio transport, (4) add LinkedIn CSV import with optional Playwright automation marked experimental. This ordering is dependency-driven: ops extraction is prerequisite for everything, bulk ops validate the shared layer and produce a reusable query engine, MCP builds on the stable ops+query foundation, and LinkedIn is fully independent and highest-risk.

The primary risks are the async/sync boundary (reqwest::blocking panics inside tokio -- mitigated by spawn_blocking), concurrent file writes from MCP (mitigated by per-file mutex), and LinkedIn account restrictions from browser automation (mitigated by making automation experimental and focusing on CSV import). All three have well-understood solutions documented in the research.

## Key Findings

### Recommended Stack

Only 3 new direct dependencies are needed. The existing stack (clap, serde, serde_yaml, chrono, ratatui, reqwest blocking, quick-xml) is unchanged.

**Core technologies:**
- **rmcp 1.1** (official Rust MCP SDK): Server framework with `#[tool]` macros, Streamable HTTP and stdio transports. Official modelcontextprotocol org SDK, released 2026-03-04
- **tokio 1** (async runtime): Required by rmcp, used only for `acrm serve` subcommand. All other commands remain synchronous
- **schemars 1** (JSON Schema generation): Required by rmcp's `#[tool]` macro to auto-generate MCP tool input schemas

**Critical version note:** Use Streamable HTTP transport (current MCP spec 2025-11-25), NOT deprecated HTTP+SSE. The PROJECT.md mentions "HTTP/SSE" but SSE was deprecated in spec version 2025-03-26.

**No new deps for bulk ops** (hand-rolled query parser, ~60 LOC). **No Rust deps for LinkedIn** (standalone Node.js Playwright script called via std::process::Command).

See: `.planning/research/STACK.md`

### Expected Features

**Must have (table stakes):**
- MCP tool registration for all existing commands (add, list, search, show, edit, log, due, delete, archive)
- stdio transport for MCP (Claude Desktop, Cursor, VS Code integration)
- Query syntax for filtering contacts (`field=value`, `field!=value`, `field~value`)
- Bulk edit, delete, archive with dry-run and confirmation prompts
- Rust-native LinkedIn CSV import replacing shell script, with dedup

**Should have (differentiators):**
- MCP resources exposing contacts as `resource://contacts/{slug}` URIs
- MCP prompt templates for CRM workflows (follow-up review, meeting prep)
- Bulk tag add/remove operators (`tags+=value`, `tags-=value`)
- JSON stdin pipe for Unix composability (`acrm search ... --format json | acrm bulk --stdin`)

**Defer (v1.x/v2+):**
- HTTP/Streamable HTTP MCP transport (add when remote agent access needed)
- Advanced query operators with date comparisons (add when users hit limits)
- LinkedIn Playwright automation (experimental, high risk of account restriction)
- Smart merge on LinkedIn re-import (interactive field-level diff)
- CalDAV integration, MCP sampling, multi-source import framework

See: `.planning/research/FEATURES.md`

### Architecture Approach

The architecture adds three new modules (mcp/, query/, linkedin/) and one critical shared layer (ops.rs) atop the existing core (store.rs, frontmatter.rs, models/). The single most important change is extracting business logic from CLI command handlers into pure functions in `ops.rs` that return `Result<T>` -- both CLI commands and MCP tools become thin wrappers around shared operations. The MCP server runs as a long-lived async process (`acrm serve`) while all other commands remain synchronous run-and-exit. Single binary strategy: tokio only initializes when `serve` is called.

**Major components:**
1. **ops.rs (NEW)** -- Pure business logic extracted from commands/ (search, add, edit, log, show, due, list, delete, archive). Prerequisite for MCP
2. **query/ (NEW)** -- Query parser (key-op-value predicates joined by AND) and filter engine applied to Vec<ContactFile> in memory. Shared by bulk CLI and MCP search
3. **mcp/ (NEW)** -- rmcp ServerHandler with `#[tool(tool_box)]`, tool definitions calling ops.rs via spawn_blocking, stdio transport setup
4. **linkedin/ (NEW)** -- Rust CSV import with dedup/change-detection; standalone Playwright JS script for automation

See: `.planning/research/ARCHITECTURE.md`

### Critical Pitfalls

1. **reqwest::blocking panics inside tokio** -- Use `spawn_blocking()` to wrap ALL sync code in MCP handlers. Never call store or sync functions directly from async context
2. **Concurrent file writes from MCP corrupt contacts** -- Add per-file mutex (`DashMap<PathBuf, Arc<Mutex<()>>>`) in MCP layer. Serialize read-modify-write per contact file
3. **Deprecated SSE transport** -- Use stdio transport initially, Streamable HTTP if HTTP needed later. Do not build on deprecated HTTP+SSE
4. **LinkedIn account restriction** -- Do not automate LinkedIn UI navigation. Focus on CSV import; if Playwright automation ships, mark experimental with rate limiting and circuit breakers
5. **Unsafe MCP write tools** -- All write tools must support `dry_run` parameter, bulk writes require `confirm: true`, hard limit of 50 contacts per MCP bulk operation

See: `.planning/research/PITFALLS.md`

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Operations Layer Extraction

**Rationale:** Hard prerequisite for both MCP and bulk ops. Pure refactor with zero behavior change -- existing tests validate preservation. Lowest risk, highest leverage.
**Delivers:** `ops.rs` with all business logic as pure functions returning `Result<T>`. All existing CLI commands refactored to thin wrappers.
**Addresses:** Architecture prerequisite (no features from FEATURES.md directly, but enables all subsequent phases)
**Avoids:** Anti-pattern of duplicating business logic between CLI and MCP handlers (Pitfall: code divergence)

### Phase 2: Bulk Operations & Query Engine

**Rationale:** Simpler than MCP (no async, no protocol complexity). Validates that the ops layer works for multi-contact operations. Query engine is reusable by MCP search tools in Phase 3.
**Delivers:** `acrm bulk` subcommand with query syntax, bulk edit/delete/archive, dry-run, confirmation, tag operators, JSON output
**Addresses:** Query syntax parser, bulk edit, bulk delete/archive, dry-run, tag add/remove, JSON output (all P1 features)
**Avoids:** Pipe chain performance trap (design pipe protocol to pass paths, not full data). Bulk without preview (require --yes for >10 contacts)

### Phase 3: MCP Server

**Rationale:** Depends on stable ops.rs (Phase 1) and benefits from query engine (Phase 2). Introduces async boundary and new protocol -- best done when core operations are proven stable.
**Delivers:** `acrm serve` with stdio transport, all CRM operations as MCP tools, contact resources, workflow prompts
**Uses:** rmcp 1.1, tokio 1, schemars 1 (all new dependencies introduced here)
**Implements:** mcp/ module with ServerHandler, spawn_blocking bridge, per-file mutex for concurrent writes
**Avoids:** reqwest::blocking panic (spawn_blocking), concurrent file corruption (per-file mutex), deprecated SSE (stdio only), unsafe write tools (dry_run + confirm parameters)

### Phase 4: LinkedIn Import & Automation

**Rationale:** Fully independent of MCP and bulk ops. Highest risk (external dependencies, account restriction concerns). CSV import is clean Rust; Playwright automation is experimental.
**Delivers:** `acrm import linkedin <file>` (Rust-native CSV import with dedup and change detection), optionally `acrm linkedin export` (Playwright automation, experimental)
**Addresses:** Rust-native LinkedIn CSV import, dedup, change detection, import summary
**Avoids:** playwright-rust dependency (standalone JS script instead), LinkedIn account restriction (rate limiting, circuit breaker, experimental label)

### Phase Ordering Rationale

- **Dependency chain:** ops.rs -> bulk ops (validates ops) -> MCP (uses ops + query) -> LinkedIn (independent)
- **Risk escalation:** Each phase adds more complexity and external dependencies than the previous
- **Incremental value:** Each phase delivers usable functionality independently. The project is shippable after any phase
- **Query engine reuse:** Building it in Phase 2 means Phase 3 MCP search gets it for free

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (MCP Server):** rmcp SDK is young (v1.1, released 2026-03-04). Tool macro API, ServerHandler patterns, and transport setup need validation against current docs. Concurrent write locking strategy needs prototyping
- **Phase 4 (LinkedIn Automation):** Playwright script for LinkedIn GDPR export page needs testing against current LinkedIn UI. Bot detection mitigation strategies need validation

Phases with standard patterns (skip research-phase):
- **Phase 1 (Ops Extraction):** Pure Rust refactoring. Extract functions, update call sites, run existing tests. No unknowns
- **Phase 2 (Bulk Operations):** Well-documented Unix CLI patterns. Simple predicate parser. Standard clap subcommand. No unknowns

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Official SDK (rmcp) verified on crates.io. Tokio and schemars are mature. Only 3 new deps |
| Features | HIGH | MCP spec is stable. Bulk ops follow Unix conventions. LinkedIn CSV format is documented |
| Architecture | HIGH | Existing codebase well-understood. spawn_blocking pattern is standard tokio. ops extraction is textbook refactoring |
| Pitfalls | HIGH (core), MEDIUM (LinkedIn) | Async/sync boundary and concurrency pitfalls are well-documented. LinkedIn bot detection specifics are less certain |

**Overall confidence:** HIGH

### Gaps to Address

- **rmcp 1.1 API stability:** SDK is 1 week old (released 2026-03-04). Pin to 1.1.x but expect possible breaking changes in tool macro syntax. Validate during Phase 3 planning
- **Concurrent write performance:** Per-file mutex strategy is sound but untested at scale with MCP. May need semaphore to limit concurrent blocking tasks. Prototype during Phase 3
- **LinkedIn GDPR export timing:** LinkedIn says CSV is ready in "10 minutes to 24 hours." Playwright automation script needs to handle this async delay gracefully. Validate during Phase 4
- **Minor version discrepancy:** STACK.md references rmcp 1.1, ARCHITECTURE.md references rmcp 0.16. Use 1.1 (latest verified on crates.io 2026-03-08)

## Sources

### Primary (HIGH confidence)
- [rmcp 1.1.0 on crates.io](https://crates.io/crates/rmcp) -- Official Rust MCP SDK, version verified
- [MCP Specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) -- Transport decisions
- [reqwest::blocking tokio incompatibility](https://github.com/seanmonstar/reqwest/issues/1233) -- Critical pitfall documentation
- [LinkedIn Export Help](https://www.linkedin.com/help/linkedin/answer/a566336/export-connections-from-linkedin) -- CSV format and fields
- Existing codebase (`src/`) -- Direct inspection of 4,700+ LOC

### Secondary (MEDIUM confidence)
- [Shuttle: Building Streamable HTTP MCP Server in Rust](https://www.shuttle.dev/blog/2025/10/29/stream-http-mcp) -- rmcp + axum integration patterns
- [MCP Best Practices (Philipp Schmid)](https://www.philschmid.de/mcp-best-practices) -- Tool design patterns
- [LinkedIn Automation Safety Guide 2026](https://www.dux-soup.com/blog/linkedin-automation-safety-guide-how-to-avoid-account-restrictions-in-2026) -- Bot detection avoidance
- [Why MCP Deprecated SSE](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/) -- Transport rationale

### Tertiary (LOW confidence)
- [Learn by Building: CRM MCP Server](https://learnbybuilding.ai/tutorial/creating-a-mcp-server-to-run-a-crm/) -- Community tutorial, patterns only

---
*Research completed: 2026-03-08*
*Ready for roadmap: yes*
