use std::fmt;

use anyhow::Result;
use colored::Colorize;

use crate::format::{self, OutputFormat};
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

pub fn run(tag: Option<&str>, output_format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let mut contacts = store::load_all_contacts(&root)?;
    contacts.sort_by(|a, b| a.contact.name.cmp(&b.contact.name));

    if let Some(tag) = tag {
        contacts.retain(|cf| cf.contact.tags.iter().any(|t| t == tag));
    }

    if contacts.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No contacts found."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    let summaries: Vec<ContactSummary> = contacts
        .iter()
        .map(|cf| {
            let c = &cf.contact;
            ContactSummary {
                name: c.name.clone(),
                company: c.company.clone(),
                status: c
                    .status
                    .as_ref()
                    .map(|s| format!("{s:?}"))
                    .unwrap_or_default(),
                tags: c.tags.clone(),
            }
        })
        .collect();

    format::output_list(&summaries, output_format, "contacts")
}
