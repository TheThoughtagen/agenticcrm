use std::fmt;

use anyhow::{Context, Result, bail};

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::models::Contact;
use crate::ops::contact::{ARRAY_FIELDS, EditResult, needs_quoting};
use crate::store;
use crate::validation;

impl fmt::Display for EditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Updated {} (fields: {}) -- {}",
            self.name,
            self.updated_fields.join(", "),
            self.path
        )
    }
}

pub fn run(name: &str, sets: &[String], format: &OutputFormat) -> Result<()> {
    if sets.is_empty() {
        bail!("No --set arguments provided. Usage: acrm edit \"name\" --set key=value");
    }

    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let mut cf = store::find_single_contact(contacts, name)?;

    let mut updated_fields = Vec::new();

    for set_arg in sets {
        let (key, value) = set_arg
            .split_once('=')
            .with_context(|| format!("Invalid --set format '{set_arg}', expected key=value"))?;

        let key = key.trim();
        let value = value.trim();

        if ARRAY_FIELDS.contains(&key) {
            // Parse comma-separated values
            let values: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cf.raw_frontmatter =
                frontmatter::update_array_field(&cf.raw_frontmatter, key, &values);
        } else {
            // For scalar fields, wrap strings in quotes if they don't look like
            // a bare YAML value (number, boolean, date, enum keyword)
            let yaml_value = if needs_quoting(value) {
                format!("\"{}\"", value.replace('"', "\\\""))
            } else {
                value.to_string()
            };
            cf.raw_frontmatter =
                frontmatter::update_field(&cf.raw_frontmatter, key, &yaml_value);
        }

        updated_fields.push(key.to_string());
    }

    // Re-parse the updated frontmatter to get the updated Contact struct
    let updated_contact: Contact = serde_yaml::from_str(&cf.raw_frontmatter)
        .context("Updated frontmatter produced invalid YAML")?;

    // Validate before writing
    let errors = validation::validate_contact(&updated_contact);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        bail!("Validation failed: {}", messages.join("; "));
    }

    cf.contact = updated_contact;

    // Write directly to the existing path (preserving filename)
    let content = store::serialize_contact_file(&cf)?;
    std::fs::write(&cf.path, &content)
        .with_context(|| format!("Failed to write {}", cf.path.display()))?;

    let result = EditResult {
        name: cf.contact.name.clone(),
        updated_fields,
        path: cf.path.display().to_string(),
    };

    format::output(&result, format)
}
