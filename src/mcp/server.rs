use rmcp::ServerHandler;
use rmcp::model::{ServerCapabilities, ServerInfo};

use super::CrmServer;

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
             delete, archive contacts, and manage follow-ups.",
        )
    }
}
