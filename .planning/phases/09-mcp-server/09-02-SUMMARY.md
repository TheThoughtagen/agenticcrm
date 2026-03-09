---
phase: 09-mcp-server
plan: 02
subsystem: api
tags: [mcp, rmcp, axum, streamable-http, resources, tools]

requires:
  - phase: 09-01
    provides: "MCP server foundation with read-only tools and stdio transport"
  - phase: 07-operations-layer
    provides: "ops functions for all CRM operations"
provides:
  - "Full MCP tool surface: 9 tools (3 read, 6 write)"
  - "contact:// resource browsing via MCP resources protocol"
  - "Streamable HTTP transport on configurable port"
  - "Concurrent write safety via tokio::sync::Mutex"
  - "Sync gating behind --allow-sync flag"
affects: [10-linkedin-import]

tech-stack:
  added: [tokio-util]
  patterns: [write-lock-before-spawn-blocking, contact-uri-scheme, streamable-http-service-factory]

key-files:
  created:
    - src/mcp/resources.rs
  modified:
    - src/mcp/tools.rs
    - src/mcp/mod.rs
    - src/main.rs
    - Cargo.toml

key-decisions:
  - "CallToolResult::error for sync-disabled message (not ErrorData) -- keeps tool callable, returns user-friendly error"
  - "contact:// URI scheme with slug from name for resource browsing"
  - "Credentials loaded from keyring in sync tool (same as CLI sync command)"
  - "StreamableHttpService with LocalSessionManager for stateful sessions"

patterns-established:
  - "Write lock pattern: acquire write_lock.lock().await BEFORE spawn_blocking for all mutation tools"
  - "Resource URI scheme: contact://{slug} where slug is lowercase hyphenated name"
  - "HTTP serve pattern: factory closure cloning CrmServer per session"

requirements-completed: [MCP-05, MCP-06, MCP-07, MCP-08, MCP-10, MCP-11, MCP-02]

duration: 7min
completed: 2026-03-09
---

# Phase 9 Plan 2: MCP Write Tools, Resources & HTTP Transport Summary

**Full MCP tool surface (9 tools), contact:// resource browsing, and Streamable HTTP transport with write safety via mutex**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-09T15:35:42Z
- **Completed:** 2026-03-09T15:42:36Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- All 6 write MCP tools implemented: add, edit, log, delete, archive, sync
- Contact resource browsing via contact:// URIs with list and read support
- Streamable HTTP transport with graceful shutdown and session management
- All write operations protected by tokio::sync::Mutex write lock
- Sync operations gated behind --allow-sync flag with user-friendly error

## Task Commits

Each task was committed atomically:

1. **Task 1: Add write tools, sync tool, and contact:// resources** - `a99d5f2` (feat)
2. **Task 2: Add Streamable HTTP transport and complete Serve command** - `5eeb2fc` (feat)

## Files Created/Modified
- `src/mcp/tools.rs` - Added 6 write tool param structs and handlers (add, edit, log, delete, archive, sync)
- `src/mcp/resources.rs` - New: contact:// resource listing and reading via ops layer
- `src/mcp/mod.rs` - Added serve_http(), resource capability, list_resources/read_resource overrides
- `src/main.rs` - Wired --http flag to serve_http vs serve_stdio
- `Cargo.toml` - Added tokio-util dependency for CancellationToken

## Decisions Made
- Used CallToolResult::error (not ErrorData) for sync-disabled message -- tool remains callable, returns descriptive error in content
- contact:// URI scheme uses slugified name (lowercase, hyphen-separated) matching filename convention
- Sync tool loads credentials from keyring (same pattern as CLI sync command via sync::config::load_credentials)
- StreamableHttpService with LocalSessionManager default for in-memory session management
- Added tokio-util for CancellationToken (required by StreamableHttpServerConfig)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing struct fields for RawResource and ListResourcesResult**
- **Found during:** Task 1 (resources.rs compilation)
- **Issue:** rmcp 1.1.1 RawResource has additional fields (title, size, icons, meta) not in research docs
- **Fix:** Added all missing Optional fields with None defaults
- **Files modified:** src/mcp/resources.rs
- **Verification:** cargo check passes
- **Committed in:** a99d5f2 (Task 1 commit)

**2. [Rule 3 - Blocking] Added tokio-util dependency for CancellationToken**
- **Found during:** Task 2 (serve_http compilation)
- **Issue:** StreamableHttpServerConfig requires CancellationToken from tokio_util crate
- **Fix:** cargo add tokio-util
- **Files modified:** Cargo.toml, Cargo.lock
- **Verification:** cargo check passes
- **Committed in:** 5eeb2fc (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes necessary for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full MCP server complete with 9 tools, resources, and dual transport
- Ready for Phase 10 (LinkedIn Import) which can use the MCP server for testing
- All 12 MCP requirements satisfied

---
*Phase: 09-mcp-server*
*Completed: 2026-03-09*
