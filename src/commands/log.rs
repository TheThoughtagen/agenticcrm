use anyhow::{Context, Result, bail};
use chrono::Local;

use crate::store;

pub fn run(name: &str, interaction_type: &str, summary: &str, notes: Option<&str>) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let name_lower = name.to_lowercase();

    let mut matches: Vec<_> = contacts
        .into_iter()
        .filter(|cf| cf.contact.name.to_lowercase().contains(&name_lower))
        .collect();

    if matches.is_empty() {
        bail!("No contact matching '{name}'");
    }
    if matches.len() > 1 {
        let names: Vec<_> = matches.iter().map(|cf| cf.contact.name.as_str()).collect();
        bail!("Multiple matches for '{name}': {}", names.join(", "));
    }

    let cf = &mut matches[0];
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

    // Update CRM fields
    cf.contact.last_contacted = Some(today);

    let path = store::write_contact(&root, cf)
        .context("Failed to write updated contact")?;

    println!("Logged {interaction_type} with {} — {}", cf.contact.name, path.display());
    Ok(())
}
