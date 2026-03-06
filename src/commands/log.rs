use std::fmt;

use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::store;

#[derive(Serialize)]
pub struct LogResult {
    pub name: String,
    pub interaction_type: String,
    pub summary: String,
    pub path: String,
}

impl fmt::Display for LogResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Logged {} with {} -- {}",
            self.interaction_type, self.name, self.path
        )
    }
}

pub fn run(
    name: &str,
    interaction_type: &str,
    summary: &str,
    notes: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let mut cf = store::find_single_contact(contacts, name)?;
    let today = Local::now().date_naive();

    // Build the interaction entry
    let mut entry = format!("\n### {today} | {interaction_type} | {summary}\n");
    if let Some(n) = notes {
        entry.push('\n');
        entry.push_str(n);
        entry.push('\n');
    }

    // Insert after "## Interaction Log" heading
    if let Some(pos) = cf.body.find("## Interaction Log") {
        let insert_at = cf.body[pos..]
            .find('\n')
            .map(|i| pos + i + 1)
            .unwrap_or(cf.body.len());
        cf.body.insert_str(insert_at, &entry);
    } else {
        cf.body.push_str("\n## Interaction Log\n");
        cf.body.push_str(&entry);
    }

    // Update CRM fields in raw frontmatter
    cf.contact.last_contacted = Some(today);
    cf.raw_frontmatter =
        frontmatter::update_field(&cf.raw_frontmatter, "last_contacted", &today.to_string());

    let path = store::write_contact(&root, &cf)
        .context("Failed to write updated contact")?;

    let result = LogResult {
        name: cf.contact.name.clone(),
        interaction_type: interaction_type.to_string(),
        summary: summary.to_string(),
        path: path.display().to_string(),
    };

    format::output(&result, output_format)
}
