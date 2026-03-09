use rmcp::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::tool_handler;

use super::CrmServer;

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
                "AgenticCRM - a personal CRM. Search, view, add, edit, log interactions, \
                 delete, archive contacts, and manage follow-ups."
                    .to_string(),
            ),
        }
    }
}
