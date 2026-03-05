use anyhow::Result;
use colored::Colorize;

use crate::store;

pub fn run(query: &str) -> Result<()> {
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
        println!("No matches for '{query}'.");
        return Ok(());
    }

    for cf in &matches {
        let c = &cf.contact;
        let company = if c.company.is_empty() {
            String::new()
        } else {
            format!(" @ {}", c.company)
        };
        println!(
            "{}{} — {}",
            c.name.bold(),
            company.dimmed(),
            cf.path.display()
        );
    }

    println!("\n{} match(es)", matches.len());
    Ok(())
}
