use std::fmt;

use anyhow::Result;
use serde::Serialize;
use uuid::Uuid;

use crate::format::{self, OutputFormat};
use crate::models::{Contact, ContactFile, Priority, Relationship, Status};
use crate::store;

#[derive(Serialize)]
pub struct AddResult {
    pub name: String,
    pub path: String,
}

impl fmt::Display for AddResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Created: {}", self.path)
    }
}

pub fn run(name: &str, output_format: &OutputFormat) -> Result<()> {
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
        etag: String::new(),
    };

    // Generate raw frontmatter from template to preserve comments
    let raw_frontmatter = store::generate_raw_frontmatter(&contact, &root)?;

    let cf = ContactFile {
        contact,
        body: "## Notes\n\n\n## Interaction Log\n".to_string(),
        path: root.join("contacts"),
        raw_frontmatter,
    };

    let path = store::write_contact(&root, &cf)?;

    let result = AddResult {
        name: cf.contact.name.clone(),
        path: path.display().to_string(),
    };

    format::output(&result, output_format)
}
