use std::fmt;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::store;

#[derive(Serialize)]
pub struct ArchiveResult {
    pub name: String,
    pub action: String,
    pub from_path: String,
    pub to_path: String,
}

impl fmt::Display for ArchiveResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({} -> {})",
            self.action, self.name, self.from_path, self.to_path
        )
    }
}

pub fn run_archive(name: &str, format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let mut cf = store::find_single_contact(contacts, name)?;

    // Update status to archived
    cf.raw_frontmatter = frontmatter::update_field(&cf.raw_frontmatter, "status", "archived");

    // Ensure archive directory exists
    let archive_dir = root.join("archive");
    std::fs::create_dir_all(&archive_dir)
        .context("Failed to create archive directory")?;

    // Write to archive/
    let filename = cf
        .path
        .file_name()
        .context("Contact file has no filename")?
        .to_owned();
    let archive_path = archive_dir.join(&filename);

    let content = format!("---\n{}---\n\n{}", cf.raw_frontmatter, cf.body);
    std::fs::write(&archive_path, &content)
        .with_context(|| format!("Failed to write {}", archive_path.display()))?;

    // Remove original from contacts/
    std::fs::remove_file(&cf.path)
        .with_context(|| format!("Failed to remove {}", cf.path.display()))?;

    let result = ArchiveResult {
        name: cf.contact.name,
        action: "Archived".to_string(),
        from_path: cf.path.display().to_string(),
        to_path: archive_path.display().to_string(),
    };

    format::output(&result, format)
}

pub fn run_unarchive(name: &str, format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let archive_dir = root.join("archive");

    let contacts = store::load_contacts_from_dir(&archive_dir)?;
    let mut cf = store::find_single_contact(contacts, name)?;

    // Update status to active
    cf.raw_frontmatter = frontmatter::update_field(&cf.raw_frontmatter, "status", "active");

    // Write to contacts/
    let filename = cf
        .path
        .file_name()
        .context("Contact file has no filename")?
        .to_owned();
    let contacts_path = root.join("contacts").join(&filename);

    let content = format!("---\n{}---\n\n{}", cf.raw_frontmatter, cf.body);
    std::fs::write(&contacts_path, &content)
        .with_context(|| format!("Failed to write {}", contacts_path.display()))?;

    // Remove from archive/
    std::fs::remove_file(&cf.path)
        .with_context(|| format!("Failed to remove {}", cf.path.display()))?;

    let result = ArchiveResult {
        name: cf.contact.name,
        action: "Unarchived".to_string(),
        from_path: cf.path.display().to_string(),
        to_path: contacts_path.display().to_string(),
    };

    format::output(&result, format)
}
