use std::fmt;

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::store;

#[derive(Serialize)]
pub struct SearchResult {
    pub name: String,
    pub company: String,
    pub path: String,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let company = if self.company.is_empty() {
            String::new()
        } else {
            format!(" @ {}", self.company)
        };
        write!(
            f,
            "{}{} -- {}",
            self.name.bold(),
            company.dimmed(),
            self.path
        )
    }
}

pub fn run(query: &str, output_format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let query_lower = query.to_lowercase();

    let matches: Vec<_> = contacts
        .into_iter()
        .filter(|cf| {
            let c = &cf.contact;
            c.name.to_lowercase().contains(&query_lower)
                || c.company.to_lowercase().contains(&query_lower)
                || c.role.to_lowercase().contains(&query_lower)
                || c.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || c.email.iter().any(|e| e.to_lowercase().contains(&query_lower))
                || cf.body.to_lowercase().contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No matches for '{query}'."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    let results: Vec<SearchResult> = matches
        .iter()
        .map(|cf| SearchResult {
            name: cf.contact.name.clone(),
            company: cf.contact.company.clone(),
            path: cf.path.display().to_string(),
        })
        .collect();

    format::output_list(&results, output_format, "match(es)")
}
