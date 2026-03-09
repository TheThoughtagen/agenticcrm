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
                    RawResource::new(format!("contact://{}", slug), cs.name)
                        .with_description(description)
                        .with_mime_type("text/markdown"),
                    None,
                )
            })
            .collect();

        Ok(ListResourcesResult::with_all_items(resources))
    }

    /// Read a single contact resource by contact:// URI.
    pub async fn mcp_read_resource(
        &self,
        params: &ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, ErrorData> {
        let uri = &params.uri;

        let slug = uri
            .strip_prefix("contact://")
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!("Invalid resource URI: '{}'. Expected contact://{{slug}}", uri),
                    None,
                )
            })?;

        let name_query = slug.replace('-', " ");

        let root = self.root.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::ops::contact::show(&root, &name_query)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(super::ops_err_to_mcp)?;

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(json, uri.clone()),
        ]))
    }
}
