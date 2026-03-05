use anyhow::Result;
use uuid::Uuid;

use crate::models::{Contact, ContactFile, Priority, Relationship, Status};
use crate::store;

pub fn run(name: &str) -> Result<()> {
    let root = store::find_crm_root()?;
    let contact = Contact {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        aliases: vec![],
        pronouns: String::new(),
        email: vec![],
        phone: vec![],
        address: vec![],
        company: String::new(),
        role: String::new(),
        industry: String::new(),
        linkedin: String::new(),
        twitter: String::new(),
        facebook: String::new(),
        instagram: String::new(),
        github: String::new(),
        website: String::new(),
        birthday: None,
        interests: vec![],
        family: vec![],
        how_we_met: String::new(),
        met_date: None,
        introduced_by: String::new(),
        relationship: Some(Relationship::Acquaintance),
        tags: vec![],
        status: Some(Status::Active),
        follow_up_cadence: String::new(),
        last_contacted: None,
        next_follow_up: None,
        priority: Some(Priority::Medium),
        source: "manual".to_string(),
        source_id: String::new(),
    };

    let cf = ContactFile {
        contact,
        body: "## Notes\n\n\n## Interaction Log\n".to_string(),
        path: root.join("contacts"),
    };

    let path = store::write_contact(&root, &cf)?;
    println!("Created: {}", path.display());
    Ok(())
}
