use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::models::{Contact, ContactFile};

const FRONTMATTER_DELIMITER: &str = "---";

/// Parse a markdown file with YAML frontmatter into Contact + body
pub fn parse_contact_file(path: &Path) -> Result<ContactFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let (contact, body) = parse_frontmatter(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", path.display()))?;

    Ok(ContactFile {
        contact,
        body,
        path: path.to_path_buf(),
    })
}

fn parse_frontmatter(content: &str) -> Result<(Contact, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        bail!("File does not start with frontmatter delimiter");
    }

    // Find the closing delimiter
    let after_first = &trimmed[FRONTMATTER_DELIMITER.len()..];
    let end = after_first
        .find(&format!("\n{FRONTMATTER_DELIMITER}"))
        .context("No closing frontmatter delimiter found")?;

    let yaml = &after_first[..end];
    let body_start = end + 1 + FRONTMATTER_DELIMITER.len();
    let body = after_first[body_start..].trim_start_matches('\n').to_string();

    let contact: Contact = serde_yaml::from_str(yaml)
        .context("Failed to parse YAML frontmatter")?;

    Ok((contact, body))
}

/// Serialize a ContactFile back to markdown with frontmatter
pub fn serialize_contact_file(cf: &ContactFile) -> Result<String> {
    let yaml = serde_yaml::to_string(&cf.contact)?;
    Ok(format!("---\n{yaml}---\n\n{}", cf.body))
}

/// Load all contacts from the contacts directory
pub fn load_all_contacts(crm_root: &Path) -> Result<Vec<ContactFile>> {
    let contacts_dir = crm_root.join("contacts");
    let mut contacts = Vec::new();

    for entry in WalkDir::new(&contacts_dir).min_depth(1).max_depth(1) {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "md") {
            match parse_contact_file(path) {
                Ok(cf) => contacts.push(cf),
                Err(e) => eprintln!("Warning: skipping {}: {e}", path.display()),
            }
        }
    }

    Ok(contacts)
}

/// Write a contact file to disk
pub fn write_contact(crm_root: &Path, cf: &ContactFile) -> Result<PathBuf> {
    let content = serialize_contact_file(cf)?;
    let path = crm_root.join("contacts").join(format!("{}.md", cf.contact.slug()));
    std::fs::write(&path, &content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(path)
}

/// Resolve the CRM root directory
pub fn find_crm_root() -> Result<PathBuf> {
    // Check env var first
    if let Ok(root) = std::env::var("ACRM_ROOT") {
        let path = PathBuf::from(root);
        if path.join("contacts").is_dir() {
            return Ok(path);
        }
    }

    // Check current directory
    let cwd = std::env::current_dir()?;
    if cwd.join("contacts").is_dir() && cwd.join("templates").is_dir() {
        return Ok(cwd);
    }

    // Default location
    if let Some(home) = dirs::home_dir() {
        let default = home.join("repos").join("agenticcrm");
        if default.join("contacts").is_dir() {
            return Ok(default);
        }
    }

    bail!(
        "Could not find CRM root. Set ACRM_ROOT or run from the agenticcrm directory."
    )
}
