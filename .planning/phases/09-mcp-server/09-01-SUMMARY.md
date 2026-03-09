---
phase: 09-mcp-server
plan: 01
subsystem: api
tags: [mcp, rmcp, tokio, axum, schemars, tracing, stdio]

# Dependency graph
requires:
  - phase: 07-operations-layer
    provides: "ops::contact functions (search, show, due) with serializable result structs"
provides:
  - "CrmServer struct with ServerHandler and tool_router"
  - "serve_stdio function for MCP stdio transport"
  - "Read-only MCP tools: search_contacts, show_contact, due_followups"
  - "OpsError to ErrorData mapping for MCP responses"
  - "Serve CLI command with --http, --port, --allow-sync flags"
affects: [09-02-PLAN, mcp-write-tools, mcp-http-transport]

# Tech tracking
tech-stack:
  added: [rmcp 1.1, tokio 1, axum 0.8, schemars 0.8, tracing 0.1, tracing-subscriber 0.3]
  patterns: [spawn_blocking bridge for sync ops, tool_router macro, ServerHandler trait]

key-files:
  created:
    - src/mcp/mod.rs
    - src/mcp/server.rs
    - src/mcp/tools.rs
  modified:
    - Cargo.toml
    - src/main.rs

key-decisions:
  - "ErrorData used directly (not McpError alias) for rmcp 1.1 API compatibility"
  - "tool_router with pub visibility for cross-module access from mod.rs"
  - "ServerInfo::new() builder pattern rather than struct literal (non-exhaustive struct)"

patterns-established:
  - "spawn_blocking bridge: all ops calls wrapped in tokio::task::spawn_blocking for async safety"
  - "Tool handler pattern: Parameters<T> extractor -> spawn_blocking(ops call) -> JSON serialize -> CallToolResult::success"
  - "Error mapping: ops_err_to_mcp converts OpsError variants to ErrorData with descriptive messages"

requirements-completed: [MCP-01, MCP-03, MCP-04, MCP-09, MCP-12]

# Metrics
duration: 6min
completed: 2026-03-09
---

# Phase 9 Plan 1: MCP Server Foundation Summary

**MCP server with rmcp 1.1 stdio transport and three read-only tools (search, show, due) wrapping ops layer via spawn_blocking**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-09T15:21:26Z
- **Completed:** 2026-03-09T15:27:18Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- MCP server foundation with CrmServer struct, ServerHandler, and tool_router
- Three read-only MCP tools wrapping ops::contact functions via spawn_blocking
- Serve CLI command wired with stdio transport
- Write lock and allow_sync fields ready for plan 02's write tools

## Task Commits

Each task was committed atomically:

1. **Task 1: Add MCP dependencies and create CrmServer with ServerHandler** - `a94aa09` (feat)
2. **Task 2: Implement read-only MCP tools and wire Serve CLI command** - `a485137` (feat)

## Files Created/Modified
- `Cargo.toml` - Added rmcp, tokio, axum, schemars, tracing dependencies
- `src/mcp/mod.rs` - CrmServer struct, serve_stdio function, ops_err_to_mcp error mapping
- `src/mcp/server.rs` - ServerHandler impl with get_info() returning server capabilities
- `src/mcp/tools.rs` - tool_router with search_contacts, show_contact, due_followups tools
- `src/main.rs` - Added mod mcp, Serve command variant, and match arm

## Decisions Made
- Used `ErrorData` directly instead of `McpError` alias since rmcp 1.1 exports it as `rmcp::ErrorData`
- Used `ServerInfo::new()` builder pattern since `ServerInfo` struct is non-exhaustive in rmcp 1.1
- Set `tool_router(vis = "pub")` so mod.rs can call `Self::tool_router()` from the `new()` constructor
- Tool handlers return `Result<CallToolResult, ErrorData>` for explicit error handling rather than auto-converting `String`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted to rmcp 1.1 actual API surface**
- **Found during:** Task 1 and Task 2
- **Issue:** Research examples used `McpError` (an alias), struct literals for `ServerInfo`/`Implementation` (non-exhaustive), and `#[tool(param)]` attribute (not the actual API)
- **Fix:** Used `ErrorData` directly, `ServerInfo::new()` builder, `Parameters<T>` extractor pattern from rmcp test suite
- **Files modified:** src/mcp/mod.rs, src/mcp/server.rs, src/mcp/tools.rs
- **Verification:** `cargo build` succeeds, `cargo run -- serve --help` shows correct flags
- **Committed in:** a94aa09, a485137

---

**Total deviations:** 1 auto-fixed (1 blocking - API surface mismatch)
**Impact on plan:** Necessary adaptation to actual rmcp 1.1 API. No scope creep.

## Issues Encountered
- rmcp 1.1 API differs from research examples in several ways (McpError vs ErrorData, non-exhaustive structs, tool param syntax). Resolved by reading rmcp test files in cargo registry to find correct patterns.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CrmServer has write_lock and allow_sync fields ready for plan 02's write tools
- Tool handler pattern established and proven with three read-only tools
- HTTP transport flag accepted but not yet implemented (plan 02)

---
*Phase: 09-mcp-server*
*Completed: 2026-03-09*
