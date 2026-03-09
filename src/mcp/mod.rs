pub mod server;
pub mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::{ErrorData, ServiceExt};
use tokio::sync::Mutex;

use crate::ops::OpsError;

/// CRM MCP server wrapping the ops layer for agent access.
#[derive(Clone)]
pub struct CrmServer {
    pub root: PathBuf,
    pub write_lock: Arc<Mutex<()>>,
    pub allow_sync: bool,
    pub tool_router: ToolRouter<Self>,
}

impl CrmServer {
    pub fn new(root: PathBuf, allow_sync: bool) -> Self {
        Self {
            root,
            write_lock: Arc::new(Mutex::new(())),
            allow_sync,
            tool_router: Self::tool_router(),
        }
    }
}

/// Start the MCP server on stdio transport.
pub async fn serve_stdio(server: CrmServer) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting acrm MCP server on stdio");

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

/// Map OpsError to rmcp ErrorData for MCP tool responses.
pub fn ops_err_to_mcp(e: OpsError) -> ErrorData {
    match e {
        OpsError::NotFound(msg) => ErrorData::internal_error(
            format!("Contact not found: {msg}"),
            None,
        ),
        OpsError::AmbiguousMatch { query, matches } => ErrorData::invalid_params(
            format!("Multiple contacts match '{query}': {matches}"),
            None,
        ),
        OpsError::ValidationFailed(msg) => ErrorData::invalid_params(
            format!("Validation failed: {msg}"),
            None,
        ),
        OpsError::SyncError(msg) => ErrorData::internal_error(
            format!("Sync error: {msg}"),
            None,
        ),
        OpsError::Io(e) => ErrorData::internal_error(
            format!("IO error: {e}"),
            None,
        ),
        OpsError::Internal(msg) => ErrorData::internal_error(msg, None),
    }
}
