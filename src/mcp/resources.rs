use rmcp::model::{
    Annotated, ListResourcesResult, RawResource, ReadResourceRequestParams,
    ReadResourceResult, Resource, ResourceContents,
};
use rmcp::ErrorData;

use super::CrmServer;

impl CrmServer {
    /// List all contacts as MCP resources with contact:// URIs.
    pub async fn mcp_list_resources(&self) -> Result<ListResourcesResult, ErrorData> {
        let root = self.root.clone();
        let contacts = tokio::task::spawn_blocking(move || {
            crate::ops::contact::list(&root, None)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(super::ops_err_to_mcp)?;

        let resources: Vec<Resource> = contacts
            .into_iter()
            .map(|cs| {
                // Build slug from name: lowercase, hyphenated, alphanumeric only
                let slug = cs
                    .name
                    .to_lowercase()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("-");

                let description = if cs.company.is_empty() {
                    cs.status.clone()
                } else {
                    format!("{} - {}", cs.company, cs.status)
                };

                Annotated::new(
                    RawResource {
                        uri: format!("contact://{}", slug),
                        name: cs.name,
                        title: None,
                        description: Some(description),
                        mime_type: Some("text/markdown".to_string()),
                        size: None,
                        icons: None,
                        meta: None,
                    },
                    None,
                )
            })
            .collect();

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    /// Read a single contact resource by contact:// URI.
    pub async fn mcp_read_resource(
        &self,
        params: &ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &params.uri;

        // Parse contact:// URI to extract slug
        let slug = uri
            .strip_prefix("contact://")
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!("Invalid resource URI: '{}'. Expected contact://{{slug}}", uri),
                    None,
                )
            })?;

        // Convert slug to name query (replace hyphens with spaces)
        let name_query = slug.replace('-', " ");

        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::show(&root, &name_query)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(super::ops_err_to_mcp)?;

        // Serialize the contact detail as JSON for the resource content
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(json, uri.clone()),
        ]))
    }
}
