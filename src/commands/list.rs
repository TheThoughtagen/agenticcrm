use anyhow::Result;
use colored::Colorize;

use crate::store;

pub fn run(tag: Option<&str>) -> Result<()> {
    let root = store::find_crm_root()?;
    let mut contacts = store::load_all_contacts(&root)?;
    contacts.sort_by(|a, b| a.contact.name.cmp(&b.contact.name));

    if let Some(tag) = tag {
        contacts.retain(|cf| cf.contact.tags.iter().any(|t| t == tag));
    }

    if contacts.is_empty() {
        println!("No contacts found.");
        return Ok(());
    }

    for cf in &contacts {
        let c = &cf.contact;
        let company = if c.company.is_empty() {
            String::new()
        } else {
            format!(" @ {}", c.company)
        };
        let status = c
            .status
            .as_ref()
            .map(|s| format!("{s:?}"))
            .unwrap_or_default();
        let tags = if c.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", c.tags.join(", "))
        };

        println!(
            "{}{} {} {}",
            c.name.bold(),
            company.dimmed(),
            status.dimmed(),
            tags.dimmed()
        );
    }

    println!("\n{} contacts", contacts.len());
    Ok(())
}
