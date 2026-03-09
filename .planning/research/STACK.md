# Technology Stack

**Project:** AgenticCRM v1.2 -- MCP Server, Bulk Ops & LinkedIn Automation
**Researched:** 2026-03-08
**Overall confidence:** HIGH

## Scope

This research covers ONLY new dependencies for v1.2 features. The existing stack (clap 4, serde, serde_yaml, chrono, anyhow, ratatui 0.29, reqwest blocking, quick-xml, calcard, keyring) is validated and unchanged.

## Recommended Stack Additions

### MCP Server

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| rmcp | 1.1 | Official Rust MCP SDK -- server framework, tool macros, transport | Official SDK from modelcontextprotocol org. Released 2026-03-04. 22+ releases since March 2025. Supports Streamable HTTP (current spec) and stdio. |
| tokio | 1 (full) | Async runtime required by rmcp | rmcp is built on tokio -- no alternative. Required for Streamable HTTP transport and async tool handlers |
| schemars | 1 | JSON Schema generation for MCP tool input definitions | rmcp's `#[tool]` macro uses schemars to auto-generate input schemas for tool parameters |

**rmcp feature flags:**
```toml
rmcp = { version = "1.1", features = [
    "server",                           # Server-side MCP functionality
    "macros",                           # #[tool] and #[prompt] derive macros
    "transport-streamable-http-server", # HTTP transport (current MCP spec)
    "transport-io",                     # stdio transport for local agent use
] }
```

Axum 0.8 ships as a transitive dependency of rmcp's `transport-streamable-http-server` feature. No need to add it explicitly unless mounting custom routes alongside MCP.

### Bulk Operations

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| (no new deps) | -- | Query parsing and pipeline I/O | Simple `field=value` grammar parsed with existing `regex` crate. JSON pipe I/O uses existing `serde_json`. No query engine needed at <10K contacts |

### LinkedIn Automation

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| (no new Rust deps) | -- | Subprocess orchestration | `std::process::Command` spawns a standalone Playwright script. No Rust browser crate needed |
| Node.js + Playwright | external | Browser automation for LinkedIn CSV export | External runtime dependency. User installs separately. Explicitly experimental feature |

## Critical Architecture Decision: Async vs Blocking

The existing codebase is entirely synchronous using `reqwest::blocking`. The rmcp MCP SDK requires tokio async. These CANNOT coexist on the same thread -- `reqwest::blocking` panics inside a tokio runtime.

**Solution: Separate binary entry point.**

- `acrm add/list/search/edit/log/...` -- synchronous, no tokio, completely unchanged
- `acrm serve` -- new subcommand with `#[tokio::main]` async runtime
- `acrm serve --stdio` -- stdio transport variant for local agents

The `serve` command wraps all existing synchronous store/command logic via `tokio::task::spawn_blocking()`. This means:
- Zero refactoring of existing commands
- MCP tool handlers call the same `store.rs` and `commands/` functions
- No async migration of reqwest or any existing code

This is the cleanest integration path. The alternative (migrating the entire codebase to async) would be a massive refactor with zero user benefit for CLI usage.

## Complete Cargo.toml Additions

```toml
[dependencies]
# MCP server (NEW for v1.2)
rmcp = { version = "1.1", features = [
    "server", "macros",
    "transport-streamable-http-server",
    "transport-io",
] }
tokio = { version = "1", features = ["full"] }
schemars = "1"

# All other deps unchanged from v1.1
```

**Total new direct dependencies:** 3 (rmcp, tokio, schemars)

## MCP Transport Choice: Streamable HTTP (NOT deprecated SSE)

The PROJECT.md mentions "HTTP/SSE" but the MCP specification deprecated the HTTP+SSE transport in version 2025-03-26, replacing it with **Streamable HTTP**. Streamable HTTP is the current standard (spec version 2025-11-25).

Key differences:
- **Deprecated SSE:** Two endpoints (POST for client-to-server, GET /sse for server-to-client stream). Stateful connection required.
- **Streamable HTTP:** Single `/mcp` endpoint. POST for all messages. Server MAY use SSE in response body for streaming, but it is optional. Stateless-compatible.

rmcp 1.1 supports Streamable HTTP natively via `transport-streamable-http-server`. It does NOT have a feature flag for the deprecated SSE transport (that lives in the separate `rmcp-actix-web` crate, which we do not need).

**Recommend both transports:**
- `acrm serve` -- Streamable HTTP on `localhost:3000` (configurable), for network/remote agents
- `acrm serve --stdio` -- stdio transport, for local agents (Claude Desktop, Cursor, etc.)

## LinkedIn Automation: Subprocess Architecture

**Why NOT use a Rust Playwright crate:**

| Crate | Status | Problem |
|-------|--------|---------|
| `playwright` (crates.io) | v0.0.1 | Placeholder, unusable |
| `playwright-rust` (octaltree) | Functional but limited | Self-describes as "still under development and has limited functions." Spawns Node.js internally anyway |
| `pw-rs` | Community fork | Same architecture -- Node.js subprocess under the hood |

All Rust Playwright crates spawn Node.js under the hood. There is zero performance or packaging benefit. A standalone JavaScript/TypeScript Playwright script is:
- Independently testable (`node scripts/linkedin-export.js`)
- Easier to debug (browser DevTools, Playwright trace viewer)
- Maintained in a language where Playwright is a first-class citizen
- Not coupled to Rust compile cycles

**Implementation pattern:**
```
acrm linkedin export
  -> std::process::Command::new("node")
       .arg("scripts/linkedin-export.js")
       .spawn()
  -> Script outputs CSV to known path
  -> Rust reads CSV, runs existing import logic with dedup
```

**External dependency:** User must have Node.js 18+ and Playwright installed. Detect at runtime with clear error messages. Acceptable for an explicitly "experimental" feature.

## Bulk Query Syntax: No Parser Crate Needed

The query grammar is intentionally simple:
```
field=value           # exact match
field!=value          # not equal
field~=pattern        # regex match
field<date            # date comparison
field>date            # date comparison
tag:contains=value    # array contains
```

This is ~50-80 lines of hand-rolled parsing against known frontmatter field names. The existing `regex` crate handles pattern matching. Adding a parser generator (pest, nom, lalrpop) or SQL parser would be over-engineering for a fixed, small grammar with ~6 operators and ~15 known fields.

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| MCP SDK | rmcp 1.1 (official) | rust-mcp-sdk 0.x | rust-mcp-sdk is community-maintained. rmcp is the official modelcontextprotocol org SDK with stronger protocol compliance guarantees and faster spec tracking |
| MCP transport | Streamable HTTP | Deprecated HTTP+SSE | SSE transport deprecated in MCP spec 2025-03-26. Build on current standard |
| HTTP framework | axum (via rmcp) | actix-web (via rmcp-actix-web) | rmcp bundles axum natively. actix-web requires a separate adapter crate. No benefit to adding complexity |
| Async runtime | tokio | async-std / smol | rmcp requires tokio. No choice |
| Query parsing | Hand-rolled | pest / nom / sqlparser | Grammar is trivial. Parser generators add compile-time and complexity for ~60 lines of code |
| LinkedIn browser | Node.js Playwright subprocess | playwright-rust crate | Rust crates are immature and spawn Node.js internally anyway. Direct script is simpler and independently testable |
| LinkedIn browser | Playwright | Puppeteer / Selenium | Playwright has best headless mode, auto-wait, multi-browser. De facto standard for 2026 browser automation |

## What NOT to Add

| Temptation | Why Avoid |
|------------|-----------|
| Full async migration of existing CLI | Massive refactor with zero user benefit. Only `acrm serve` needs async |
| Database (SQLite, sled) for bulk queries | Flat-file is the product differentiator. Linear scan of <10K contacts is <50ms |
| REST/GraphQL API alongside MCP | MCP is the agent protocol. Adding REST splits maintenance for no audience |
| Embedded V8/Deno for Playwright | Enormous binary size. Node.js is already required for Playwright |
| reqwest async migration | Breaking change to sync commands. spawn_blocking bridge is simpler |
| tracing crate | Useful but not blocking. Can add later. eprintln/log patterns sufficient for now |
| Web UI framework | Out of scope per PROJECT.md constraints |

## Dependency Risk Assessment

| Dependency | Risk | Mitigation |
|------------|------|------------|
| rmcp 1.1 | MEDIUM -- young SDK (v1.1, org since Mar 2025), API surface may shift | Pin to `1.1.x`. MCP protocol itself is stable. Official SDK has strong incentive for API stability. Worst case: update tool macro annotations |
| tokio 1 | LOW -- most-used Rust async runtime, stable ABI for years | Standard choice, no risk |
| schemars 1 | LOW -- mature, widely used for JSON Schema | Stable API |
| Node.js/Playwright (external) | MEDIUM -- external runtime dep, user must install | Feature is explicitly experimental. Runtime detection with clear errors. Script is standalone |

## Sources

- [rmcp crate 1.1.0 on crates.io](https://crates.io/crates/rmcp) -- HIGH confidence, verified latest version 2026-03-08
- [rmcp docs.rs feature flags](https://docs.rs/crate/rmcp/latest) -- HIGH confidence, all features enumerated
- [modelcontextprotocol/rust-sdk on GitHub](https://github.com/modelcontextprotocol/rust-sdk) -- HIGH confidence, official repository
- [MCP Transports Specification (2025-03-26)](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports) -- HIGH confidence, official spec documenting SSE deprecation
- [Why MCP Deprecated SSE](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/) -- MEDIUM confidence, community analysis with protocol rationale
- [Building Streamable HTTP MCP Server in Rust (Shuttle)](https://www.shuttle.dev/blog/2025/10/29/stream-http-mcp) -- MEDIUM confidence, working tutorial with rmcp + axum
- [reqwest::blocking docs](https://docs.rs/reqwest/latest/reqwest/blocking/index.html) -- HIGH confidence, documents tokio runtime incompatibility
- [playwright-rust on GitHub](https://github.com/octaltree/playwright-rust) -- HIGH confidence, verified limited/development status

---

*Stack research: 2026-03-08 -- v1.2 milestone (MCP, Bulk Ops, LinkedIn)*
*Previous research (v1.1 milestone): 2026-03-07*
*Previous research (v1.0 milestone): 2026-03-05*
