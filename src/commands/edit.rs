use std::fmt;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::models::Contact;
use crate::store;
use crate::validation;

/// Known array fields in the contact schema
const ARRAY_FIELDS: &[&str] = &[
    "email",
    "phone",
    "address",
    "aliases",
    "interests",
    "family",
    "tags",
];

#[derive(Serialize)]
pub struct EditResult {
    pub name: String,
    pub updated_fields: Vec<String>,
    pub path: String,
}

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

/// Determine if a value needs YAML quoting.
/// Values that are plain YAML scalars (numbers, booleans, dates, known enums) don't need quotes.
fn needs_quoting(value: &str) -> bool {
    // Empty string needs quoting
    if value.is_empty() {
        return true;
    }

    // Already quoted
    if value.starts_with('"') && value.ends_with('"') {
        return false;
    }

    // Known bare YAML values that don't need quotes
    let bare_values = [
        "true", "false", "null", "~",
        "active", "dormant", "lost-touch", "archived",
        "friend", "colleague", "client", "mentor", "mentee", "acquaintance", "family", "other",
        "high", "medium", "low",
        "weekly", "biweekly", "monthly", "quarterly", "yearly",
        "manual", "linkedin", "carddav",
    ];

    if bare_values.contains(&value.to_lowercase().as_str()) {
        return false;
    }

    // Numbers don't need quotes
    if value.parse::<f64>().is_ok() {
        return false;
    }

    // Dates (YYYY-MM-DD) don't need quotes
    if value.len() == 10 && value.chars().nth(4) == Some('-') && value.chars().nth(7) == Some('-') {
        if value.parse::<chrono::NaiveDate>().is_ok() {
            return false;
        }
    }

    true
}
