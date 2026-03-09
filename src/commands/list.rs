use std::fmt;

use colored::Colorize;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::ContactSummary;
use crate::store;

impl fmt::Display for ContactSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let company = if self.company.is_empty() {
            String::new()
        } else {
            format!(" @ {}", self.company)
        };
        let tags = if self.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.tags.join(", "))
        };
        write!(
            f,
            "{}{} {} {}",
            self.name.bold(),
            company.dimmed(),
            self.status.dimmed(),
            tags.dimmed()
        )
    }
}

pub fn run(tag: Option<&str>, output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let summaries = ops::contact::list(&root, tag)?;

    if summaries.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No contacts found."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    format::output_list(&summaries, output_format, "contacts")
}
