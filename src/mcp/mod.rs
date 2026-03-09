pub mod resources;
pub mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{
    ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ErrorData, ServerHandler, ServiceExt, tool_handler};
use tokio::sync::Mutex;

use crate::ops::OpsError;

/// CRM MCP server wrapping the ops layer for agent access.
#[derive(Clone)]
pub struct CrmServer {
    pub root: PathBuf,
    pub write_lock: Arc<Mutex<()>>,
    pub allow_sync: bool,
    tool_router: ToolRouter<Self>,
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

#[tool_handler]
impl ServerHandler for CrmServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(
            "AgenticCRM - a personal CRM. Search, view, add, edit, log interactions, \
             delete, archive contacts, and sync with iCloud. Browse contacts as resources.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        self.mcp_list_resources().await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        self.mcp_read_resource(&request).await
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

/// Start the MCP server on Streamable HTTP transport.
pub async fn serve_http(server: CrmServer, port: u16) -> anyhow::Result<()> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager,
        tower::{StreamableHttpServerConfig, StreamableHttpService},
    };
    use tokio_util::sync::CancellationToken;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let ct = CancellationToken::new();
    let session_manager = Arc::new(LocalSessionManager::default());

    let config = StreamableHttpServerConfig {
        cancellation_token: ct.clone(),
        ..Default::default()
    };

    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        session_manager,
        config,
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    tracing::info!("MCP HTTP server listening on http://127.0.0.1:{}/mcp", port);

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            ct.cancel();
        })
        .await?;

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
