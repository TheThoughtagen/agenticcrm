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

    let formats = ["%d %b %Y", "%b %d, %Y", "%Y-%m-%d", "%m/%d/%y", "%m/%d/%Y"];

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

                // Compute fields that would be set from CSV data
                let company = row.company.trim();
                if !company.is_empty() {
                    fields_set.push("company".to_string());
                }
                let role = row.position.trim();
                if !role.is_empty() {
                    fields_set.push("role".to_string());
                }
                fields_set.push("source".to_string());
                fields_set.push("relationship".to_string());
                let email = row.email_address.trim();
                if !email.is_empty() {
                    fields_set.push("email".to_string());
                }
                if parse_connected_on(&row.connected_on).is_some() {
                    fields_set.push("met_date".to_string());
                }
                fields_set.push("tags".to_string());

                let path_str = if !dry_run {
                    // Actually create the contact and write frontmatter
                    let add_result = super::contact::add(root, &name)
                        .map_err(|e| OpsError::Internal(e.to_string()))?;

                    let path = std::path::PathBuf::from(&add_result.path);
                    let mut cf = store::parse_contact_file(&path)
                        .map_err(|e| OpsError::Internal(e.to_string()))?;

                    // Company
                    if !company.is_empty() {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "company",
                            &format!("\"{}\"", company),
                        );
                    }

                    // Role (from Position)
                    if !role.is_empty() {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "role",
                            &format!("\"{}\"", role),
                        );
                    }

                    // Source
                    cf.raw_frontmatter =
                        frontmatter::update_field(&cf.raw_frontmatter, "source", "linkedin");

                    // Relationship
                    cf.raw_frontmatter =
                        frontmatter::update_field(&cf.raw_frontmatter, "relationship", "colleague");

                    // Email
                    if !email.is_empty() {
                        cf.raw_frontmatter = frontmatter::update_array_field(
                            &cf.raw_frontmatter,
                            "email",
                            &[email.to_string()],
                        );
                    }

                    // Met date
                    if let Some(date) = parse_connected_on(&row.connected_on) {
                        cf.raw_frontmatter = frontmatter::update_field(
                            &cf.raw_frontmatter,
                            "met_date",
                            &date.to_string(),
                        );
                    }

                    // Tags
                    cf.raw_frontmatter = frontmatter::update_array_field(
                        &cf.raw_frontmatter,
                        "tags",
                        &["linkedin".to_string()],
                    );

                    // Re-parse and write
                    let updated_contact: crate::models::Contact =
                        serde_yaml::from_str(&cf.raw_frontmatter).map_err(|e| {
                            OpsError::Internal(format!("Updated frontmatter invalid YAML: {e}"))
                        })?;
                    cf.contact = updated_contact;

                    let content = store::serialize_contact_file(&cf)
                        .map_err(|e| OpsError::Internal(e.to_string()))?;
                    std::fs::write(&cf.path, &content)?;

                    add_result.path
                } else {
                    // Predict the path without creating anything on disk
                    let filename = format!(
                        "contacts/{}.md",
                        name.to_lowercase().replace(' ', "-")
                    );
                    root.join(&filename).display().to_string()
                };

                created.push(ImportChange {
                    name: name.clone(),
                    path: path_str,
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
                } else {
                    skipped.push(ImportSkip {
                        name: name.clone(),
                        reason: "no changes needed".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a temp CRM root with contacts/, templates/, and a minimal template.
    fn setup_test_root() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        fs::create_dir_all(root.join("contacts")).unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();

        let template = r#"---
id: "{{uuid}}"
name: ""
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: acquaintance
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: ""
---

## Notes


## Interaction Log
"#;
        fs::write(root.join("templates/contact.md"), template).unwrap();
        (tmp, root)
    }

    fn write_csv(root: &std::path::Path, content: &str) -> std::path::PathBuf {
        let csv_path = root.join("test.csv");
        fs::write(&csv_path, content).unwrap();
        csv_path
    }

    fn make_contact(root: &std::path::Path, filename: &str, frontmatter: &str) {
        let content = format!("---\n{}\n---\n\n## Notes\n\n## Interaction Log\n", frontmatter);
        fs::write(root.join("contacts").join(filename), content).unwrap();
    }

    const CSV_HEADER: &str = "First Name,Last Name,Email Address,Company,Position,Connected On";

    #[test]
    fn test_new_contact_is_created() {
        let (_tmp, root) = setup_test_root();
        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,Acme Inc,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.created.len(), 1);
        assert_eq!(result.created[0].name, "Jane Doe");
        assert!(result.created[0].fields.contains(&"company".to_string()));
        assert!(result.created[0].fields.contains(&"role".to_string()));
        assert!(result.created[0].fields.contains(&"source".to_string()));
        assert!(result.created[0].fields.contains(&"email".to_string()));
        assert!(result.created[0].fields.contains(&"met_date".to_string()));
        assert!(result.created[0].fields.contains(&"tags".to_string()));
        assert!(result.created[0].fields.contains(&"relationship".to_string()));

        // Verify the file on disk
        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("source: linkedin"));
        assert!(content.contains("relationship: colleague"));
        assert!(content.contains("\"linkedin\""));
        assert!(content.contains("\"Acme Inc\""));
        assert!(content.contains("\"Engineer\""));
        assert!(content.contains("jane@example.com"));
        assert!(content.contains("met_date: 2024-01-15"));
    }

    #[test]
    fn test_existing_contact_matched_by_name_gets_fill_empty_updates() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-111"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,Acme,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.created.len(), 0);
        assert!(result.updated[0].fields.contains(&"company".to_string()));
        assert!(result.updated[0].fields.contains(&"role".to_string()));
        assert!(result.updated[0].fields.contains(&"email".to_string()));
        assert!(result.updated[0].fields.contains(&"tags".to_string()));

        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("\"Acme\""));
        assert!(content.contains("\"Engineer\""));
        assert!(content.contains("jane@example.com"));
        assert!(content.contains("\"linkedin\""));
    }

    #[test]
    fn test_existing_contact_matched_by_email() {
        let (_tmp, root) = setup_test_root();

        // Name differs but email matches
        make_contact(&root, "janet-doe.md", r#"id: "aaa-222"
name: "Janet Doe"
aliases: []
pronouns: ""
email:
  - "jane@example.com"
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,BigCo,Manager,01 Feb 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        // Should match Janet Doe by email, update her
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.created.len(), 0);
        assert!(result.updated[0].fields.contains(&"company".to_string()));
    }

    #[test]
    fn test_non_empty_fields_never_overwritten_detected_changes_reported() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-333"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: "OldCo"
role: "Director"
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,NewCo,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();

        // Company and role should NOT be overwritten
        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("\"OldCo\""));
        assert!(content.contains("\"Director\""));
        assert!(!content.contains("\"NewCo\""));
        assert!(!content.contains("\"Engineer\""));

        // But detected changes should be reported
        assert!(result.detected_changes.len() >= 2);
        let company_change = result
            .detected_changes
            .iter()
            .find(|dc| dc.field == "company")
            .unwrap();
        assert_eq!(company_change.crm_value, "OldCo");
        assert_eq!(company_change.linkedin_value, "NewCo");

        let role_change = result
            .detected_changes
            .iter()
            .find(|dc| dc.field == "role")
            .unwrap();
        assert_eq!(role_change.crm_value, "Director");
        assert_eq!(role_change.linkedin_value, "Engineer");
    }

    #[test]
    fn test_ambiguous_match_skipped() {
        let (_tmp, root) = setup_test_root();

        // Two contacts with same name
        make_contact(&root, "jane-doe-1.md", r#"id: "aaa-444"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: "Co1"
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        make_contact(&root, "jane-doe-2.md", r#"id: "aaa-555"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: "Co2"
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,,Acme,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("ambiguous"));
        assert_eq!(result.created.len(), 0);
        assert_eq!(result.updated.len(), 0);
    }

    #[test]
    fn test_email_array_merged_with_dedup() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-666"
name: "Jane Doe"
aliases: []
pronouns: ""
email:
  - "old@example.com"
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,new@example.com,,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.updated.len(), 1);
        assert!(result.updated[0].fields.contains(&"email".to_string()));

        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("old@example.com"));
        assert!(content.contains("new@example.com"));
    }

    #[test]
    fn test_email_dedup_case_insensitive() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-777"
name: "Jane Doe"
aliases: []
pronouns: ""
email:
  - "Jane@Example.com"
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags:
  - "linkedin"
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        // No email should be added (case-insensitive dedup), and linkedin tag already present
        // So no fields changed -> counted as skipped, not updated
        assert_eq!(result.updated.len(), 0);
        assert_eq!(result.skipped.len(), 1, "no-change match should be skipped");
        assert!(result.skipped[0].reason.contains("no changes"), "reason should mention no changes");
    }

    #[test]
    fn test_linkedin_tag_added_if_not_present() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-888"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags:
  - "rust"
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,,,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.updated.len(), 1);
        assert!(result.updated[0].fields.contains(&"tags".to_string()));

        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("\"rust\""));
        assert!(content.contains("\"linkedin\""));
    }

    #[test]
    fn test_linkedin_tag_deduped_if_already_present() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "aaa-999"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags:
  - "linkedin"
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,,,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        // linkedin tag already present, nothing else to update -> counted as skipped
        assert_eq!(result.updated.len(), 0);
        assert_eq!(result.skipped.len(), 1, "no-change match should be skipped");
        assert!(result.skipped[0].reason.contains("no changes"), "reason should mention no changes");
    }

    #[test]
    fn test_dry_run_returns_result_but_no_files_written() {
        let (_tmp, root) = setup_test_root();
        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,Acme,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, true).unwrap();
        assert!(result.dry_run);
        assert_eq!(result.created.len(), 1);
        assert_eq!(result.created[0].name, "Jane Doe");

        // Dry-run must NOT create any file on disk
        assert!(
            !root.join("contacts/jane-doe.md").exists(),
            "dry_run should not create contact file on disk"
        );
    }

    #[test]
    fn test_dry_run_no_update_written() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "bbb-111"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,,Acme,Engineer,"),
        );

        let result = import_linkedin(&root, &csv_path, true).unwrap();
        assert!(result.dry_run);
        assert_eq!(result.updated.len(), 1);

        // File should NOT have been changed
        let content = fs::read_to_string(root.join("contacts/jane-doe.md")).unwrap();
        assert!(content.contains("company: \"\""));
        assert!(!content.contains("\"Acme\""));
    }

    #[test]
    fn test_empty_email_falls_back_to_name_matching() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "bbb-222"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        // No email in CSV
        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,,Acme,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        // Should match by name
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.created.len(), 0);
    }

    #[test]
    fn test_malformed_rows_produce_warnings() {
        let (_tmp, root) = setup_test_root();

        // CSV with header and one malformed row (missing columns)
        let csv_content = format!("{CSV_HEADER}\nJane,Doe,jane@example.com,Acme,Engineer,15 Jan 2024\nBad Row Only");
        let csv_path = write_csv(&root, &csv_content);

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        // First row should succeed
        assert_eq!(result.created.len(), 1);
        // Second row should produce a warning
        assert!(result.warnings.len() >= 1);
    }

    #[test]
    fn test_case_insensitive_name_matching() {
        let (_tmp, root) = setup_test_root();

        make_contact(&root, "jane-doe.md", r#"id: "ccc-111"
name: "Jane Doe"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: ""
role: ""
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date:
introduced_by: ""
relationship: friend
tags: []
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: manual
source_id: ""
etag: """#);

        // CSV with different casing
        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJANE,DOE,,Acme,,"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.created.len(), 0);
    }

    #[test]
    fn test_parse_connected_on_formats() {
        // %d %b %Y
        assert_eq!(
            parse_connected_on("15 Jan 2024"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        // %b %d, %Y
        assert_eq!(
            parse_connected_on("Jan 15, 2024"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        // %Y-%m-%d
        assert_eq!(
            parse_connected_on("2024-01-15"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        // %m/%d/%Y
        assert_eq!(
            parse_connected_on("01/15/2024"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        // %m/%d/%y
        assert_eq!(
            parse_connected_on("01/15/24"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        // Empty
        assert_eq!(parse_connected_on(""), None);
        // Invalid
        assert_eq!(parse_connected_on("not a date"), None);
    }

    #[test]
    fn test_reimport_no_changes_counted_as_skipped() {
        let (_tmp, root) = setup_test_root();

        // Contact with ALL LinkedIn fields already populated
        make_contact(&root, "jane-doe.md", r#"id: "ddd-111"
name: "Jane Doe"
aliases: []
pronouns: ""
email:
  - "jane@example.com"
phone: []
address: []
company: "Acme Inc"
role: "Engineer"
industry: ""
linkedin: ""
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""
birthday:
interests: []
family: []
how_we_met: ""
met_date: 2024-01-15
introduced_by: ""
relationship: friend
tags:
  - "linkedin"
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium
source: linkedin
source_id: ""
etag: """#);

        let csv_path = write_csv(
            &root,
            &format!("{CSV_HEADER}\nJane,Doe,jane@example.com,Acme Inc,Engineer,15 Jan 2024"),
        );

        let result = import_linkedin(&root, &csv_path, false).unwrap();
        assert_eq!(result.skipped.len(), 1, "no-change re-import should be skipped");
        assert!(
            result.skipped[0].reason.contains("no changes"),
            "skip reason should mention no changes, got: {}",
            result.skipped[0].reason
        );
        assert_eq!(result.updated.len(), 0, "should not be in updated list");
        assert_eq!(result.created.len(), 0, "should not be in created list");
    }

    #[test]
    fn test_summary_counts() {
        let result = ImportResult {
            created: vec![ImportChange {
                name: "A".into(),
                path: "a".into(),
                fields: vec![],
            }],
            updated: vec![
                ImportChange {
                    name: "B".into(),
                    path: "b".into(),
                    fields: vec![],
                },
                ImportChange {
                    name: "C".into(),
                    path: "c".into(),
                    fields: vec![],
                },
            ],
            skipped: vec![],
            warnings: vec!["warn".into()],
            detected_changes: vec![],
            dry_run: false,
        };

        let (c, u, s, w) = result.summary_counts();
        assert_eq!((c, u, s, w), (1, 2, 0, 1));
    }
}
