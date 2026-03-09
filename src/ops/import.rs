use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;

use super::OpsError;

// ── LinkedIn CSV row ────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct LinkedInRow {
    #[serde(rename = "First Name")]
    pub first_name: String,
    #[serde(rename = "Last Name")]
    pub last_name: String,
    #[serde(rename = "Email Address")]
    pub email_address: String,
    #[serde(rename = "Company")]
    pub company: String,
    #[serde(rename = "Position")]
    pub position: String,
    #[serde(rename = "Connected On")]
    pub connected_on: String,
}

// ── Result types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub created: Vec<ImportChange>,
    pub updated: Vec<ImportChange>,
    pub skipped: Vec<ImportSkip>,
    pub warnings: Vec<String>,
    pub detected_changes: Vec<DetectedChange>,
    pub dry_run: bool,
}

impl ImportResult {
    pub fn summary_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.created.len(),
            self.updated.len(),
            self.skipped.len(),
            self.warnings.len(),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ImportChange {
    pub name: String,
    pub path: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportSkip {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct DetectedChange {
    pub name: String,
    pub field: String,
    pub crm_value: String,
    pub linkedin_value: String,
}

// ── CSV reader ──────────────────────────────────────────────────────────────

fn read_linkedin_csv(path: &Path) -> Result<(Vec<LinkedInRow>, Vec<String>), OpsError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| OpsError::Internal(format!("Failed to open CSV: {e}")))?;

    let mut rows = Vec::new();
    let mut warnings = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => rows.push(row),
            Err(e) => warnings.push(format!("Row {}: {e}", i + 1)),
        }
    }

    Ok((rows, warnings))
}

// ── Date parser ─────────────────────────────────────────────────────────────

fn parse_connected_on(date_str: &str) -> Option<NaiveDate> {
    let date_str = date_str.trim();
    if date_str.is_empty() {
        return None;
    }

    let formats = ["%d %b %Y", "%b %d, %Y", "%Y-%m-%d", "%m/%d/%Y", "%m/%d/%y"];

    for fmt in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, fmt) {
            return Some(date);
        }
    }

    None
}

// ── Dedup matcher ───────────────────────────────────────────────────────────

use crate::models::ContactFile;

fn find_match<'a>(
    contacts: &'a [ContactFile],
    name: &str,
    email: &str,
) -> Result<Option<&'a ContactFile>, ImportSkip> {
    let name_lower = name.to_lowercase();
    let email_lower = email.to_lowercase();

    let mut matched_indices = std::collections::HashSet::new();

    // Exact name matches
    for (i, cf) in contacts.iter().enumerate() {
        if cf.contact.name.to_lowercase() == name_lower {
            matched_indices.insert(i);
        }
    }

    // Email matches (only if email non-empty)
    if !email_lower.is_empty() {
        for (i, cf) in contacts.iter().enumerate() {
            if cf.contact.email.iter().any(|e| e.to_lowercase() == email_lower) {
                matched_indices.insert(i);
            }
        }
    }

    if matched_indices.len() > 1 {
        return Err(ImportSkip {
            name: name.to_string(),
            reason: "ambiguous match: multiple contacts matched by name or email".to_string(),
        });
    }

    if let Some(&idx) = matched_indices.iter().next() {
        Ok(Some(&contacts[idx]))
    } else {
        Ok(None)
    }
}

// ── Main import function ────────────────────────────────────────────────────

use crate::frontmatter;
use crate::store;

pub fn import_linkedin(
    root: &Path,
    csv_path: &Path,
    dry_run: bool,
) -> Result<ImportResult, OpsError> {
    let (rows, mut warnings) = read_linkedin_csv(csv_path)?;
    let existing = store::load_all_contacts(root).map_err(|e| OpsError::Internal(e.to_string()))?;

    let mut created = Vec::new();
    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    let mut detected_changes = Vec::new();

    for row in &rows {
        let name = format!("{} {}", row.first_name.trim(), row.last_name.trim())
            .trim()
            .to_string();

        if name.is_empty() {
            warnings.push("Skipped row with empty name".to_string());
            continue;
        }

        match find_match(&existing, &name, &row.email_address) {
            Err(skip) => {
                skipped.push(skip);
            }
            Ok(None) => {
                // CREATE new contact
                let mut fields_set = Vec::new();

                let add_result =
                    super::contact::add(root, &name).map_err(|e| OpsError::Internal(e.to_string()))?;

                // Re-load for fresh raw_frontmatter
                let path = std::path::PathBuf::from(&add_result.path);
                let mut cf = store::parse_contact_file(&path)
                    .map_err(|e| OpsError::Internal(e.to_string()))?;

                // Company
                let company = row.company.trim();
                if !company.is_empty() {
                    cf.raw_frontmatter = frontmatter::update_field(
                        &cf.raw_frontmatter,
                        "company",
                        &format!("\"{}\"", company),
                    );
                    fields_set.push("company".to_string());
                }

                // Role (from Position)
                let role = row.position.trim();
                if !role.is_empty() {
                    cf.raw_frontmatter = frontmatter::update_field(
                        &cf.raw_frontmatter,
                        "role",
                        &format!("\"{}\"", role),
                    );
                    fields_set.push("role".to_string());
                }

                // Source
                cf.raw_frontmatter =
                    frontmatter::update_field(&cf.raw_frontmatter, "source", "linkedin");
                fields_set.push("source".to_string());

                // Relationship
                cf.raw_frontmatter =
                    frontmatter::update_field(&cf.raw_frontmatter, "relationship", "colleague");
                fields_set.push("relationship".to_string());

                // Email
                let email = row.email_address.trim();
                if !email.is_empty() {
                    cf.raw_frontmatter = frontmatter::update_array_field(
                        &cf.raw_frontmatter,
                        "email",
                        &[email.to_string()],
                    );
                    fields_set.push("email".to_string());
                }

                // Met date
                if let Some(date) = parse_connected_on(&row.connected_on) {
                    cf.raw_frontmatter = frontmatter::update_field(
                        &cf.raw_frontmatter,
                        "met_date",
                        &date.to_string(),
                    );
                    fields_set.push("met_date".to_string());
                }

                // Tags
                cf.raw_frontmatter = frontmatter::update_array_field(
                    &cf.raw_frontmatter,
                    "tags",
                    &["linkedin".to_string()],
                );
                fields_set.push("tags".to_string());

                if !dry_run {
                    // Re-parse the updated frontmatter
                    let updated_contact: crate::models::Contact =
                        serde_yaml::from_str(&cf.raw_frontmatter).map_err(|e| {
                            OpsError::Internal(format!("Updated frontmatter invalid YAML: {e}"))
                        })?;
                    cf.contact = updated_contact;

                    let content =
                        store::serialize_contact_file(&cf).map_err(|e| OpsError::Internal(e.to_string()))?;
                    std::fs::write(&cf.path, &content)?;
                }

                created.push(ImportChange {
                    name: name.clone(),
                    path: add_result.path,
                    fields: fields_set,
                });
            }
            Ok(Some(existing_cf)) => {
                // UPDATE existing contact (fill-empty-only)
                let mut cf = store::parse_contact_file(&existing_cf.path)
                    .map_err(|e| OpsError::Internal(e.to_string()))?;

                let mut fields_changed = Vec::new();

                // Company: fill-empty-only
                let li_company = row.company.trim();
                if !li_company.is_empty() {
                    if cf.contact.company.is_empty() {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "company",
                            &format!("\"{}\"", li_company),
                        );
                        fields_changed.push("company".to_string());
                    } else if cf.contact.company.to_lowercase() != li_company.to_lowercase() {
                        detected_changes.push(DetectedChange {
                            name: name.clone(),
                            field: "company".to_string(),
                            crm_value: cf.contact.company.clone(),
                            linkedin_value: li_company.to_string(),
                        });
                    }
                }

                // Role: fill-empty-only
                let li_role = row.position.trim();
                if !li_role.is_empty() {
                    if cf.contact.role.is_empty() {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "role",
                            &format!("\"{}\"", li_role),
                        );
                        fields_changed.push("role".to_string());
                    } else if cf.contact.role.to_lowercase() != li_role.to_lowercase() {
                        detected_changes.push(DetectedChange {
                            name: name.clone(),
                            field: "role".to_string(),
                            crm_value: cf.contact.role.clone(),
                            linkedin_value: li_role.to_string(),
                        });
                    }
                }

                // Source: fill-empty-only
                if cf.contact.source.is_empty() || cf.contact.source == "manual" {
                    // Don't overwrite if already set to something specific
                    // "manual" is the default, so treat it as empty for source
                }
                // Actually, per plan: source fill-empty-only. "manual" is the default,
                // let's only fill if truly empty
                if cf.contact.source.is_empty() {
                    cf.raw_frontmatter =
                        frontmatter::update_field(&cf.raw_frontmatter, "source", "linkedin");
                    fields_changed.push("source".to_string());
                } else if cf.contact.source != "linkedin"
                    && cf.contact.source != "manual"
                {
                    detected_changes.push(DetectedChange {
                        name: name.clone(),
                        field: "source".to_string(),
                        crm_value: cf.contact.source.clone(),
                        linkedin_value: "linkedin".to_string(),
                    });
                }

                // Met date: fill-empty-only
                if let Some(li_date) = parse_connected_on(&row.connected_on) {
                    if cf.contact.met_date.is_none() {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "met_date",
                            &li_date.to_string(),
                        );
                        fields_changed.push("met_date".to_string());
                    }
                    // If met_date exists but differs, we could detect it, but dates
                    // from different sources are hard to compare meaningfully
                }

                // Email: merge array (add new, dedup)
                let li_email = row.email_address.trim().to_string();
                if !li_email.is_empty() {
                    let mut emails = cf.contact.email.clone();
                    let already_has = emails
                        .iter()
                        .any(|e| e.to_lowercase() == li_email.to_lowercase());
                    if !already_has {
                        emails.push(li_email);
                        cf.raw_frontmatter =
                            frontmatter::update_array_field(&cf.raw_frontmatter, "email", &emails);
                        fields_changed.push("email".to_string());
                    }
                }

                // Tags: merge "linkedin" tag
                let mut tags = cf.contact.tags.clone();
                if !tags.iter().any(|t| t == "linkedin") {
                    tags.push("linkedin".to_string());
                    cf.raw_frontmatter =
                        frontmatter::update_array_field(&cf.raw_frontmatter, "tags", &tags);
                    fields_changed.push("tags".to_string());
                }

                if !fields_changed.is_empty() {
                    if !dry_run {
                        let updated_contact: crate::models::Contact =
                            serde_yaml::from_str(&cf.raw_frontmatter).map_err(|e| {
                                OpsError::Internal(format!(
                                    "Updated frontmatter invalid YAML: {e}"
                                ))
                            })?;
                        cf.contact = updated_contact;

                        let content = store::serialize_contact_file(&cf)
                            .map_err(|e| OpsError::Internal(e.to_string()))?;
                        std::fs::write(&cf.path, &content)?;
                    }

                    updated.push(ImportChange {
                        name: name.clone(),
                        path: cf.path.display().to_string(),
                        fields: fields_changed,
                    });
                }
            }
        }
    }

    Ok(ImportResult {
        created,
        updated,
        skipped,
        warnings,
        detected_changes,
        dry_run,
    })
}
