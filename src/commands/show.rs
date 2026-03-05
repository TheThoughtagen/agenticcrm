use anyhow::{Result, bail};
use colored::Colorize;

use crate::store;

pub fn run(name: &str) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let name_lower = name.to_lowercase();

    let matches: Vec<_> = contacts
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

    let cf = &matches[0];
    let c = &cf.contact;

    println!("{}", c.name.bold());

    if !c.company.is_empty() || !c.role.is_empty() {
        println!("{} @ {}", c.role, c.company);
    }

    if !c.email.is_empty() {
        println!("Email: {}", c.email.join(", "));
    }
    if !c.phone.is_empty() {
        println!("Phone: {}", c.phone.join(", "));
    }

    // Social links
    let socials: Vec<(&str, &str)> = vec![
        ("LinkedIn", &c.linkedin),
        ("Twitter", &c.twitter),
        ("GitHub", &c.github),
        ("Website", &c.website),
    ];
    for (label, val) in socials {
        if !val.is_empty() {
            println!("{label}: {val}");
        }
    }

    if !c.tags.is_empty() {
        println!("Tags: {}", c.tags.join(", ").dimmed());
    }
    if !c.how_we_met.is_empty() {
        println!("Met: {}", c.how_we_met);
    }
    if let Some(last) = c.last_contacted {
        println!("Last contacted: {last}");
    }
    if let Some(next) = c.next_follow_up {
        println!("Next follow-up: {next}");
    }

    // Print body (notes + interaction log)
    if !cf.body.is_empty() {
        println!("\n{}", cf.body);
    }

    Ok(())
}
