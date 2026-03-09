use std::fmt;

use colored::Colorize;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::SearchMatch;
use crate::store;

impl fmt::Display for SearchMatch {
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

pub fn run(query: &str, output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let results = ops::contact::search(&root, query)?;

    if results.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No matches for '{query}'."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    format::output_list(&results, output_format, "match(es)")
}
