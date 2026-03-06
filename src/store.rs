use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::frontmatter;
use crate::models::{Contact, ContactFile};
use crate::validation;

/// Parse a markdown file with YAML frontmatter into Contact + body
pub fn parse_contact_file(path: &Path) -> Result<ContactFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let (raw_yaml, body) = frontmatter::parse_raw_frontmatter(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", path.display()))?;

    let contact: Contact = serde_yaml::from_str(&raw_yaml)
        .with_context(|| format!("Failed to parse YAML in {}", path.display()))?;

    Ok(ContactFile {
        contact,
        body,
        path: path.to_path_buf(),
        raw_frontmatter: raw_yaml,
    })
}

/// Serialize a ContactFile back to markdown with frontmatter.
/// Uses raw_frontmatter to preserve comments and field order.
pub fn serialize_contact_file(cf: &ContactFile) -> Result<String> {
    Ok(format!("---\n{}---\n\n{}", cf.raw_frontmatter, cf.body))
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

/// Write a contact file to disk, validating before writing.
pub fn write_contact(crm_root: &Path, cf: &ContactFile) -> Result<PathBuf> {
    // Validate before writing
    let errors = validation::validate_contact(&cf.contact);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        bail!("Validation failed: {}", messages.join("; "));
    }

    let content = serialize_contact_file(cf)?;
    let path = crm_root.join("contacts").join(format!("{}.md", cf.contact.slug()));
    std::fs::write(&path, &content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(path)
}

/// Generate raw frontmatter for a new contact by filling in the template.
pub fn generate_raw_frontmatter(contact: &Contact, crm_root: &Path) -> Result<String> {
    let template_path = crm_root.join("templates").join("contact.md");
    let template_content = std::fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read template at {}", template_path.display()))?;

    let (raw, _) = frontmatter::parse_raw_frontmatter(&template_content)
        .context("Failed to parse template frontmatter")?;

    // Fill in values from the contact
    let mut fm = raw;
    fm = frontmatter::update_field(&fm, "id", &format!("\"{}\"", contact.id));
    fm = frontmatter::update_field(&fm, "name", &format!("\"{}\"", contact.name));

    if !contact.company.is_empty() {
        fm = frontmatter::update_field(&fm, "company", &format!("\"{}\"", contact.company));
    }
    if !contact.role.is_empty() {
        fm = frontmatter::update_field(&fm, "role", &format!("\"{}\"", contact.role));
    }
    if let Some(ref status) = contact.status {
        let status_str = serde_yaml::to_string(status)?;
        fm = frontmatter::update_field(&fm, "status", status_str.trim());
    }
    if let Some(ref rel) = contact.relationship {
        let rel_str = serde_yaml::to_string(rel)?;
        fm = frontmatter::update_field(&fm, "relationship", rel_str.trim());
    }
    if let Some(ref priority) = contact.priority {
        let pri_str = serde_yaml::to_string(priority)?;
        fm = frontmatter::update_field(&fm, "priority", pri_str.trim());
    }
    if !contact.source.is_empty() {
        fm = frontmatter::update_field(&fm, "source", &contact.source);
    }

    // Ensure trailing newline
    if !fm.ends_with('\n') {
        fm.push('\n');
    }

    Ok(fm)
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
