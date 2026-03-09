use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use rmcp::schemars;
use rmcp::ErrorData;
use schemars::JsonSchema;

use super::{ops_err_to_mcp, CrmServer};

// ── Read tool params ────────────────────────────────────────────────────────

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

// ── Write tool params ───────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct AddParams {
    #[schemars(description = "Full name of the new contact, e.g. 'Jane Smith'")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct EditParams {
    #[schemars(description = "Contact name or partial match")]
    pub name: String,
    #[schemars(description = "Field-value pairs, e.g. ['company=Acme', 'role=Engineer']")]
    pub sets: Vec<String>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct LogParams {
    #[schemars(description = "Contact name or partial match")]
    pub name: String,
    #[schemars(description = "One of: coffee, call, email, message, conference, meeting, lunch, intro")]
    pub interaction_type: String,
    #[schemars(description = "Short summary of the interaction")]
    pub summary: String,
    #[schemars(description = "Optional detailed notes")]
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct DeleteParams {
    #[schemars(description = "Contact name to delete permanently")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct ArchiveParams {
    #[schemars(description = "Contact name to archive")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
pub struct SyncParams {
    #[schemars(description = "Sync direction: 'pull', 'push', or 'both'")]
    pub direction: String,
    #[schemars(description = "Force sync even when server has newer changes")]
    #[serde(default)]
    pub force: bool,
    #[schemars(description = "Preview changes without writing")]
    #[serde(default)]
    pub dry_run: bool,
}

// ── Tool router ─────────────────────────────────────────────────────────────

#[tool_router(vis = "pub(crate)")]
impl CrmServer {
    // ── Read tools ──────────────────────────────────────────────────────

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

    // ── Write tools ─────────────────────────────────────────────────────

    #[tool(description = "Add a new contact by full name")]
    async fn add_contact(
        &self,
        Parameters(params): Parameters<AddParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::add(&root, &params.name)
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

    #[tool(description = "Edit a contact's fields using key=value pairs")]
    async fn edit_contact(
        &self,
        Parameters(params): Parameters<EditParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::edit(&root, &params.name, &params.sets)
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

    #[tool(description = "Log an interaction with a contact")]
    async fn log_interaction(
        &self,
        Parameters(params): Parameters<LogParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::log_interaction(
                &root,
                &params.name,
                &params.interaction_type,
                &params.summary,
                params.notes.as_deref(),
            )
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

    #[tool(description = "Delete a contact permanently by name")]
    async fn delete_contact(
        &self,
        Parameters(params): Parameters<DeleteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::confirm_delete(&root, &params.name)
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

    #[tool(description = "Archive a contact (sets status to archived, moves to archive/)")]
    async fn archive_contact(
        &self,
        Parameters(params): Parameters<ArchiveParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::archive(&root, &params.name)
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

    #[tool(description = "Sync contacts with iCloud via CardDAV (requires --allow-sync)")]
    async fn sync_contacts(
        &self,
        Parameters(params): Parameters<SyncParams>,
    ) -> Result<CallToolResult, ErrorData> {
        // Check if sync is allowed
        if !self.allow_sync {
            return Ok(CallToolResult::error(vec![
                rmcp::model::Content::text(
                    "Sync is disabled. Start the server with --allow-sync to enable sync operations.",
                ),
            ]));
        }

        // Load credentials from keyring (same pattern as CLI sync command)
        let (apple_id, app_password) = crate::sync::config::load_credentials()
            .map_err(|e| ErrorData::internal_error(
                format!("Failed to load sync credentials: {}. Run `acrm sync setup` first.", e),
                None,
            ))?;

        let _guard = self.write_lock.lock().await;
        let root = self.root.clone();
        let direction = params.direction.clone();
        let force = params.force;
        let dry_run = params.dry_run;

        let result = tokio::task::spawn_blocking(move || {
            let credentials = crate::ops::sync::SyncCredentials {
                apple_id,
                app_password,
            };
            let opts = crate::ops::sync::SyncOpts { force, dry_run };
            let filter = crate::sync::filter::SyncFilter::default();

            match direction.as_str() {
                "pull" => {
                    let r = crate::ops::sync::sync_pull(&root, &credentials, &filter, &opts)?;
                    serde_json::to_string_pretty(&r)
                        .map_err(|e| crate::ops::OpsError::Internal(e.to_string()))
                }
                "push" => {
                    let r = crate::ops::sync::sync_push(&root, &credentials, &filter, &opts)?;
                    serde_json::to_string_pretty(&r)
                        .map_err(|e| crate::ops::OpsError::Internal(e.to_string()))
                }
                "both" => {
                    let (pull_r, push_r) = crate::ops::sync::sync_bidirectional(
                        &root, &credentials, &filter, &filter, &opts,
                    )?;
                    let combined = serde_json::json!({
                        "pull": pull_r,
                        "push": push_r,
                    });
                    serde_json::to_string_pretty(&combined)
                        .map_err(|e| crate::ops::OpsError::Internal(e.to_string()))
                }
                other => Err(crate::ops::OpsError::ValidationFailed(
                    format!("Invalid sync direction '{}'. Use 'pull', 'push', or 'both'.", other),
                )),
            }
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(ops_err_to_mcp)?;

        Ok(CallToolResult::success(vec![
            rmcp::model::Content::text(result),
        ]))
    }
}
