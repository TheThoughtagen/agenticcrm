# Phase 9: MCP Server - Research

**Researched:** 2026-03-09
**Domain:** MCP (Model Context Protocol) server implementation in Rust
**Confidence:** HIGH

## Summary

Phase 9 exposes all CRM operations as MCP tools via `acrm serve`, enabling AI agents to discover and use search, show, add, edit, log, delete, archive, due, and sync operations programmatically. The server needs two transports: stdio (default, for local agent integration like Claude Desktop) and Streamable HTTP (`--http` flag, for remote access).

The project already has a clean ops layer (`src/ops/`) that extracts all business logic from CLI handlers, returning serializable result structs. This is the foundation -- MCP tool handlers will call ops functions, serialize results as `Content::text()` JSON, and return `CallToolResult`. The rmcp crate v1.1.0 (the official Rust MCP SDK) provides the `#[tool]`, `#[tool_router]`, and `#[tool_handler]` macros that eliminate most boilerplate.

**Primary recommendation:** Use rmcp 1.1.x with `server`, `macros`, `transport-io`, and `transport-streamable-http-server` features. Wrap all ops calls in `tokio::task::spawn_blocking` since the ops layer uses synchronous file I/O and reqwest::blocking. Use `tokio::sync::Mutex` as a global write lock to satisfy MCP-12 (concurrent file safety).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MCP-01 | MCP server runs via `acrm serve` with stdio transport | rmcp `transport-io` feature, `stdio()` transport function |
| MCP-02 | MCP server supports Streamable HTTP transport | rmcp `transport-streamable-http-server` feature + axum |
| MCP-03 | Agent can search contacts via MCP tool | Wrap `ops::contact::search()` in `#[tool]` handler |
| MCP-04 | Agent can view full contact details via MCP tool | Wrap `ops::contact::show()` in `#[tool]` handler |
| MCP-05 | Agent can add a new contact via MCP tool | Wrap `ops::contact::add()` in `#[tool]` handler |
| MCP-06 | Agent can edit contact fields via MCP tool | Wrap `ops::contact::edit()` in `#[tool]` handler |
| MCP-07 | Agent can log an interaction via MCP tool | Wrap `ops::contact::log_interaction()` in `#[tool]` handler |
| MCP-08 | Agent can delete or archive a contact via MCP tool | Wrap `ops::contact::confirm_delete()` and `archive()` in `#[tool]` handlers |
| MCP-09 | Agent can list contacts due for follow-up via MCP tool | Wrap `ops::contact::due()` in `#[tool]` handler |
| MCP-10 | Agent can trigger sync push/pull via MCP tool | Wrap `ops::sync::sync_pull/push()` with configurable permission flag |
| MCP-11 | Contacts exposed as MCP resources with `contact://` URIs | Implement `list_resources()` and `read_resource()` on ServerHandler |
| MCP-12 | Concurrent MCP requests don't corrupt contact files | `tokio::sync::Mutex` write lock + `spawn_blocking` for all ops calls |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rmcp | 1.1.x | MCP SDK (server, tools, resources, transport) | Official Rust MCP SDK from modelcontextprotocol org |
| tokio | 1.x | Async runtime for MCP server | Required by rmcp; already used by reqwest internally |
| axum | 0.8.x | HTTP framework for Streamable HTTP transport | Required by rmcp's `transport-streamable-http-server` |
| schemars | 0.8.x | JSON Schema generation for tool parameters | Required by rmcp `#[tool]` macro for parameter schemas |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1.x | Structured logging for MCP server | Debug logging, error tracing in server mode |
| tracing-subscriber | 0.3.x | Log output to stderr (stdio transport uses stdout) | Server startup, request logging |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rmcp | rust-mcp-sdk | Less mature, not official; rmcp is the modelcontextprotocol org SDK |
| axum | actix-web (via rmcp-actix-web) | rmcp examples and docs default to axum; less friction |

**Installation (add to Cargo.toml):**
```toml
rmcp = { version = "1.1", features = ["server", "macros", "transport-io", "transport-streamable-http-server"] }
tokio = { version = "1", features = ["full"] }
axum = "0.8"
schemars = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── main.rs              # CLI entry point (unchanged, add Serve command)
├── mcp/
│   ├── mod.rs           # Module declaration, CrmServer struct
│   ├── server.rs        # ServerHandler impl, get_info(), initialize()
│   ├── tools.rs         # #[tool_router] impl with all tool methods
│   └── resources.rs     # Resource listing and reading (contact:// URIs)
├── ops/                 # Existing ops layer (unchanged)
├── commands/            # Existing CLI commands (unchanged)
└── ...
```

### Pattern 1: Tool Router with spawn_blocking Bridge

**What:** Each MCP tool method wraps an ops function call inside `spawn_blocking` to avoid blocking the tokio runtime (ops layer uses synchronous file I/O).

**When to use:** All tool handlers that call into the ops layer.

**Example:**
```rust
use rmcp::{tool, tool_router, tool_handler, ServerHandler, McpError};
use rmcp::model::*;
use schemars::JsonSchema;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CrmServer {
    root: std::path::PathBuf,
    write_lock: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct SearchParams {
    #[schemars(description = "Search query (matches name, company, tags, email, notes)")]
    pub query: String,
}

#[tool_router]
impl CrmServer {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self {
            root,
            write_lock: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search contacts by name, company, tag, or free text")]
    async fn search_contacts(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let root = self.root.clone();
        let results = tokio::task::spawn_blocking(move || {
            crate::ops::contact::search(&root, &params.query)
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}
```

### Pattern 2: Write Lock for Mutation Operations

**What:** All write operations (add, edit, log, delete, archive) acquire a `tokio::sync::Mutex` before calling ops, preventing concurrent file corruption.

**When to use:** Any tool that modifies contact files.

**Example:**
```rust
#[tool(description = "Add a new contact")]
async fn add_contact(
    &self,
    Parameters(params): Parameters<AddParams>,
) -> Result<CallToolResult, McpError> {
    let root = self.root.clone();
    let _guard = self.write_lock.lock().await;  // serialize writes
    let result = tokio::task::spawn_blocking(move || {
        crate::ops::contact::add(&root, &params.name)
    })
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}
```

### Pattern 3: ServerHandler with tool_handler Macro

**What:** The `#[tool_handler]` macro wires the tool router to the ServerHandler trait automatically.

**Example:**
```rust
#[tool_handler]
impl ServerHandler for CrmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "acrm".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            instructions: Some(
                "AgenticCRM - a personal contact relationship manager. \
                 Search, view, add, edit, and manage contacts and interactions."
                    .to_string(),
            ),
        }
    }

    // Resource methods for contact:// URIs
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        // List all contacts as contact://{slug} resources
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        // Parse contact:// URI, load contact, return as text
    }
}
```

### Pattern 4: Dual Transport Entry Point

**What:** `acrm serve` uses stdio by default; `acrm serve --http` starts Streamable HTTP on a configurable port.

**Example:**
```rust
// In main.rs, add to Commands enum:
/// Start MCP server
Serve {
    /// Use HTTP transport instead of stdio
    #[arg(long)]
    http: bool,
    /// HTTP port (default: 3000)
    #[arg(long, default_value = "3000")]
    port: u16,
    /// Allow sync operations (disabled by default for safety)
    #[arg(long)]
    allow_sync: bool,
}

// In match block:
Commands::Serve { http, port, allow_sync } => {
    // Build tokio runtime and run MCP server
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let server = CrmServer::new(root, allow_sync);
        if http {
            mcp::serve_http(server, port).await
        } else {
            mcp::serve_stdio(server).await
        }
    })
}
```

### Anti-Patterns to Avoid
- **Calling reqwest::blocking inside tokio runtime directly:** The reqwest blocking client panics if called from within a tokio runtime context. Always wrap in `spawn_blocking`.
- **Fine-grained per-file locks:** Over-engineered for a personal CRM with <10K files. A single write mutex is simpler and sufficient.
- **Returning raw OpsError to MCP clients:** Map OpsError variants to appropriate McpError types (NotFound -> resource_not_found, ValidationFailed -> invalid_params, etc.).
- **Making the whole codebase async:** Only `acrm serve` needs tokio. Keep CLI synchronous (per project decision in STATE.md).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MCP protocol handling | Custom JSON-RPC parser | rmcp crate | Protocol is complex with capability negotiation, sessions, etc. |
| Tool schema generation | Manual JSON Schema | schemars derive + rmcp `#[tool]` macro | Auto-generates from struct definitions |
| Streamable HTTP transport | Custom SSE/HTTP handler | rmcp `transport-streamable-http-server` + axum | Session management, SSE streaming handled automatically |
| JSON-RPC error codes | Manual error code mapping | `McpError::internal_error()`, `McpError::invalid_params()` | Standard MCP error codes |

**Key insight:** The rmcp macros (`#[tool]`, `#[tool_router]`, `#[tool_handler]`) eliminate 80%+ of boilerplate. The ops layer already returns serializable structs, so tool handlers are thin wrappers.

## Common Pitfalls

### Pitfall 1: reqwest::blocking Panic in Tokio Runtime
**What goes wrong:** `reqwest::blocking::Client` panics when called inside a tokio runtime because it tries to create its own runtime.
**Why it happens:** The sync operations (especially CardDAV sync) use `reqwest::blocking`.
**How to avoid:** Always wrap ops calls in `tokio::task::spawn_blocking()`. This runs the closure on a dedicated thread pool outside the async runtime.
**Warning signs:** Panic with message "Cannot start a runtime from within a runtime."

### Pitfall 2: Logging to Stdout Breaks Stdio Transport
**What goes wrong:** Any output to stdout corrupts the MCP JSON-RPC stream when using stdio transport.
**Why it happens:** MCP stdio transport uses stdout for protocol messages. `println!()` or `eprintln!()` to stdout breaks framing.
**How to avoid:** Use `tracing` with stderr writer: `tracing_subscriber::fmt().with_writer(std::io::stderr).init()`. Remove any `println!()` in MCP code paths.
**Warning signs:** "Parse error" from MCP client, garbled responses.

### Pitfall 3: Forgetting to Enable Resources Capability
**What goes wrong:** `list_resources` and `read_resource` calls silently fail or are never sent by the client.
**Why it happens:** MCP uses capability negotiation. If `enable_resources()` is not in ServerCapabilities, the client won't call resource methods.
**How to avoid:** Include `.enable_resources()` in the `ServerCapabilities::builder()` chain.
**Warning signs:** Resources don't appear in client, no errors.

### Pitfall 4: MCP Delete Without Confirmation
**What goes wrong:** Agent deletes contacts without user confirmation, losing data.
**Why it happens:** MCP tools are non-interactive; there's no way to prompt for confirmation.
**How to avoid:** Use the two-phase delete pattern from ops (`find_delete_target` + `confirm_delete`). The MCP tool should use `confirm_delete` directly since the agent itself decides to call it. Consider adding a `confirm: bool` parameter or making delete always require explicit intent in the tool description.
**Warning signs:** Accidental deletions reported by users.

### Pitfall 5: Sync Permission Without Guard
**What goes wrong:** Agent triggers iCloud sync accidentally, pushing/pulling data without user intent.
**Why it happens:** Sync modifies remote data on iCloud -- more dangerous than local file operations.
**How to avoid:** Gate sync tools behind `--allow-sync` flag on `acrm serve`. When not enabled, sync tools return an error explaining they're disabled.
**Warning signs:** Unexpected iCloud sync activity.

## Code Examples

### Complete Tool Parameter Structs
```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    #[schemars(description = "Search query matching name, company, tags, email, or notes")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowParams {
    #[schemars(description = "Contact name or partial name match")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddParams {
    #[schemars(description = "Full name of the new contact, e.g. 'Jane Smith'")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EditParams {
    #[schemars(description = "Contact name or partial match")]
    pub name: String,
    #[schemars(description = "Field-value pairs to set, e.g. ['company=Acme', 'role=Engineer']")]
    pub sets: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LogParams {
    #[schemars(description = "Contact name or partial match")]
    pub name: String,
    #[schemars(description = "Interaction type: coffee, call, email, message, conference, meeting, lunch, intro")]
    pub interaction_type: String,
    #[schemars(description = "Short summary of the interaction")]
    pub summary: String,
    #[schemars(description = "Optional detailed notes")]
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteParams {
    #[schemars(description = "Contact name or partial match to delete permanently")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ArchiveParams {
    #[schemars(description = "Contact name or partial match to archive")]
    pub name: String,
}

// DueParams: no parameters needed (lists all due contacts)

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SyncParams {
    #[schemars(description = "Sync direction: 'pull', 'push', or 'both'")]
    pub direction: String,
    #[schemars(description = "Force sync (ignore ETags/conflicts)")]
    #[serde(default)]
    pub force: bool,
    #[schemars(description = "Preview only, don't write changes")]
    #[serde(default)]
    pub dry_run: bool,
}
```

### OpsError to McpError Mapping
```rust
fn ops_err_to_mcp(e: crate::ops::OpsError) -> McpError {
    match e {
        OpsError::NotFound(msg) => McpError::resource_not_found(
            "contact_not_found",
            Some(serde_json::json!({ "detail": msg })),
        ),
        OpsError::AmbiguousMatch { query, matches } => McpError::invalid_params(
            &format!("Multiple contacts match '{}': {}", query, matches),
            None,
        ),
        OpsError::ValidationFailed(msg) => McpError::invalid_params(&msg, None),
        _ => McpError::internal_error(e.to_string(), None),
    }
}
```

### Resource Implementation for contact:// URIs
```rust
async fn list_resources(
    &self,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<RoleServer>,
) -> Result<ListResourcesResult, McpError> {
    let root = self.root.clone();
    let contacts = tokio::task::spawn_blocking(move || {
        crate::ops::contact::list(&root, None)
    })
    .await
    .map_err(|e| McpError::internal_error(e.to_string(), None))?
    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let resources: Vec<_> = contacts.iter().map(|c| {
        let slug = c.name.to_lowercase().split_whitespace()
            .collect::<Vec<_>>().join("-");
        RawResource::new(
            format!("contact://{}", slug),
            &c.name,
        ).no_annotation()
    }).collect();

    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
        meta: None,
    })
}
```

### Stdio Server Startup
```rust
pub async fn serve_stdio(server: CrmServer) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)  // CRITICAL: not stdout
        .with_ansi(false)
        .init();

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### HTTP Server Startup
```rust
use rmcp::transport::StreamableHttpService;
use rmcp::handler::server::session::LocalSessionManager;

pub async fn serve_http(server: CrmServer, port: u16) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("MCP HTTP server listening on http://{}/mcp", addr);

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;

    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| HTTP+SSE transport | Streamable HTTP transport | MCP spec 2025-03-26 | SSE deprecated; use `transport-streamable-http-server` feature |
| Manual JSON-RPC handling | rmcp `#[tool]` macros | rmcp 0.8+ | 80% less boilerplate |
| Custom server handler | `#[tool_router]` + `#[tool_handler]` | rmcp 0.11+ | Auto-wires tool routing to ServerHandler |

**Deprecated/outdated:**
- HTTP+SSE transport: Deprecated in MCP spec (March 2025); out of scope per REQUIREMENTS.md
- Manual `ServerHandler` tool routing: Replaced by `#[tool_handler]` macro in recent rmcp versions

## Open Questions

1. **rmcp 1.1.x API stability**
   - What we know: rmcp 1.1.0 released 2026-03-04, only 5 days old at project start. STATE.md flags this as a concern.
   - What's unclear: Whether minor API changes will land in 1.1.x patches.
   - Recommendation: Pin to `rmcp = "1.1"` (allows 1.1.x patches). If compilation breaks, check rmcp changelog. The core patterns (tool_router, ServerHandler) have been stable since 0.8+.

2. **McpError variant availability**
   - What we know: `McpError::internal_error()` and `McpError::resource_not_found()` exist. `McpError::invalid_params()` is likely available.
   - What's unclear: Exact method signatures may vary between versions.
   - Recommendation: During implementation, check `McpError` docs/autocomplete. Fall back to `McpError::internal_error()` for any unmapped errors.

3. **Sync credential handling in MCP context**
   - What we know: SyncCredentials must be passed by caller (ops never loads from keyring). CLI loads from keyring. MCP server needs a credential source.
   - What's unclear: Whether MCP server should load from keyring at startup, from env vars, or from config.
   - Recommendation: Load from keyring at startup (same as CLI) when `--allow-sync` is enabled. The MCP server runs on the user's machine, so keyring access is appropriate.

## Sources

### Primary (HIGH confidence)
- [rmcp 1.1.0 docs](https://docs.rs/rmcp/latest/rmcp/) - Module structure, traits, types
- [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) - Official repository, README examples
- Project source code: `src/ops/contact.rs`, `src/ops/sync.rs`, `src/ops/error.rs` - ops layer API

### Secondary (MEDIUM confidence)
- [Building MCP Servers in Rust with rmcp](https://rup12.net/posts/write-your-mcps-in-rust/) - Complete guide with tool_router, tool_handler patterns
- [Streamable HTTP MCP Server in Rust (Shuttle)](https://www.shuttle.dev/blog/2025/10/29/stream-http-mcp) - HTTP transport setup with axum
- [MCP Specification - Resources](https://modelcontextprotocol.io/specification/2025-06-18/server/resources) - Custom URI scheme support
- [Build MCP Servers in Rust (MCPcat)](https://mcpcat.io/guides/building-mcp-server-rust/) - Additional examples

### Tertiary (LOW confidence)
- McpError variant names and exact method signatures (inferred from examples, not verified against 1.1.0 API docs)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - rmcp is the official SDK, decided in STATE.md, version confirmed on crates.io/docs.rs
- Architecture: HIGH - ops layer is clean and already returns serializable structs; tool handler pattern is well-documented
- Pitfalls: HIGH - reqwest::blocking panic and stdout corruption are well-known Rust async issues; documented in STATE.md blockers
- Resource implementation: MEDIUM - contact:// URI pattern follows MCP spec but exact rmcp API for resources may need adjustment during implementation

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (rmcp is new but core patterns are stabilizing)
