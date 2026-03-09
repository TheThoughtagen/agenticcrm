use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use rmcp::schemars;
use rmcp::ErrorData;
use schemars::JsonSchema;

use super::{ops_err_to_mcp, CrmServer};

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct SearchParams {
    #[schemars(description = "Search query matching name, company, tags, email, or notes")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct ShowParams {
    #[schemars(description = "Contact name or partial name match")]
    pub name: String,
}

#[tool_router(vis = "pub(crate)")]
impl CrmServer {
    #[tool(description = "Search contacts by name, company, tag, email, or free text")]
    async fn search_contacts(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let root = self.root.clone();
        let results = tokio::task::spawn_blocking(move || {
            crate::ops::contact::search(&root, &params.query)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(ops_err_to_mcp)?;

        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![
            rmcp::model::Content::text(json),
        ]))
    }

    #[tool(description = "Show full details for a contact by name or partial match")]
    async fn show_contact(
        &self,
        Parameters(params): Parameters<ShowParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::show(&root, &params.name)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(ops_err_to_mcp)?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![
            rmcp::model::Content::text(json),
        ]))
    }

    #[tool(description = "List contacts due for follow-up, sorted by most overdue first")]
    async fn due_followups(&self) -> Result<CallToolResult, ErrorData> {
        let root = self.root.clone();
        let results = tokio::task::spawn_blocking(move || crate::ops::contact::due(&root))
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .map_err(ops_err_to_mcp)?;

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![
                rmcp::model::Content::text("No contacts due for follow-up"),
            ]));
        }

        let json = serde_json::to_string_pretty(&results)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![
            rmcp::model::Content::text(json),
        ]))
    }
}
