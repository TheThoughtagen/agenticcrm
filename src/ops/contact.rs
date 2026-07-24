use std::path::Path;

use serde::Serialize;
use uuid::Uuid;

use super::OpsError;
use crate::frontmatter;
use crate::models::{Contact, ContactFile, Priority, Relationship, Status};
use crate::store;
use crate::validation;

// Re-export chrono types needed by callers
pub use chrono::NaiveDate;
use chrono::{Duration, Local, Months};

/// Known array fields in the contact schema
pub const ARRAY_FIELDS: &[&str] = &[
    "email",
    "phone",
    "address",
    "aliases",
    "interests",
    "family",
    "tags",
];

// ── Result structs ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AddResult {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ContactSummary {
    pub name: String,
    pub company: String,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchMatch {
    pub name: String,
    pub company: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ContactDetail {
    #[serde(flatten)]
    pub contact: Contact,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct EditResult {
    pub name: String,
    pub updated_fields: Vec<String>,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct LogResult {
    pub name: String,
    pub interaction_type: String,
    pub summary: String,
    pub path: String,
    pub last_contacted: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_follow_up: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DueContact {
    pub name: String,
    pub next_follow_up: String,
    pub last_contacted: String,
    pub overdue_days: i64,
}

#[derive(Debug, Serialize)]
pub struct DeleteTarget {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResult {
    pub name: String,
    pub path: String,
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct ArchiveResult {
    pub name: String,
    pub action: String,
    pub from_path: String,
    pub to_path: String,
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Find a single contact by fuzzy name match, returning OpsError on failure.
fn find_contact(contacts: Vec<ContactFile>, name: &str) -> Result<ContactFile, OpsError> {
    let name_lower = name.to_lowercase();

    // An exact (case-insensitive) name match always wins, even if the name
    // also happens to be a substring of some other contact's name (e.g.
    // "Dad" is a literal substring of "Mudadi") — callers that already know
    // the precise name (like the TUI, which resolves a contact by table
    // selection before ever calling into ops::contact) shouldn't hit a
    // spurious ambiguity error over an unrelated coincidental match.
    if let Some(pos) = contacts
        .iter()
        .position(|cf| cf.contact.name.to_lowercase() == name_lower)
    {
        let mut contacts = contacts;
        return Ok(contacts.remove(pos));
    }

    let mut matches: Vec<_> = contacts
        .into_iter()
        .filter(|cf| cf.contact.name.to_lowercase().contains(&name_lower))
        .collect();

    if matches.is_empty() {
        return Err(OpsError::NotFound(name.to_string()));
    }
    if matches.len() > 1 {
        let names: Vec<_> = matches.iter().map(|cf| cf.contact.name.clone()).collect();
        return Err(OpsError::AmbiguousMatch {
            query: name.to_string(),
            matches: names.join(", "),
        });
    }

    Ok(matches.remove(0))
}

/// Map an anyhow::Error to OpsError::Internal
fn internal(e: anyhow::Error) -> OpsError {
    OpsError::Internal(e.to_string())
}

// ── Ops functions ───────────────────────────────────────────────────────────

/// Create a new contact with the given name.
pub fn add(root: &Path, name: &str) -> Result<AddResult, OpsError> {
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

    let raw_frontmatter = store::generate_raw_frontmatter(&contact, root).map_err(internal)?;

    let cf = ContactFile {
        contact,
        body: "## Notes\n\n\n## Interaction Log\n".to_string(),
        path: root.join("contacts"),
        raw_frontmatter,
    };

    let path = store::write_contact(root, &cf).map_err(internal)?;

    Ok(AddResult {
        name: cf.contact.name.clone(),
        path: path.display().to_string(),
    })
}

/// List all contacts, optionally filtered by tag.
pub fn list(root: &Path, tag: Option<&str>) -> Result<Vec<ContactSummary>, OpsError> {
    let mut contacts = store::load_all_contacts(root).map_err(internal)?;
    contacts.sort_by(|a, b| a.contact.name.cmp(&b.contact.name));

    if let Some(tag) = tag {
        contacts.retain(|cf| cf.contact.tags.iter().any(|t| t == tag));
    }

    let summaries = contacts
        .iter()
        .map(|cf| {
            let c = &cf.contact;
            ContactSummary {
                name: c.name.clone(),
                company: c.company.clone(),
                status: c
                    .status
                    .as_ref()
                    .map(|s| format!("{s:?}"))
                    .unwrap_or_default(),
                tags: c.tags.clone(),
            }
        })
        .collect();

    Ok(summaries)
}

/// Search contacts by query across name, company, role, tags, email, and notes.
pub fn search(root: &Path, query: &str) -> Result<Vec<SearchMatch>, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let query_lower = query.to_lowercase();

    let matches: Vec<_> = contacts
        .into_iter()
        .filter(|cf| {
            let c = &cf.contact;
            c.name.to_lowercase().contains(&query_lower)
                || c.company.to_lowercase().contains(&query_lower)
                || c.role.to_lowercase().contains(&query_lower)
                || c.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                || c.email.iter().any(|e| e.to_lowercase().contains(&query_lower))
                || cf.body.to_lowercase().contains(&query_lower)
        })
        .collect();

    let results = matches
        .iter()
        .map(|cf| SearchMatch {
            name: cf.contact.name.clone(),
            company: cf.contact.company.clone(),
            path: cf.path.display().to_string(),
        })
        .collect();

    Ok(results)
}

/// Show full details for a contact matched by name.
pub fn show(root: &Path, name: &str) -> Result<ContactDetail, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let cf = find_contact(contacts, name)?;

    Ok(ContactDetail {
        contact: cf.contact,
        body: cf.body,
    })
}

/// Edit a contact's fields using key=value pairs.
pub fn edit(root: &Path, name: &str, sets: &[String]) -> Result<EditResult, OpsError> {
    if sets.is_empty() {
        return Err(OpsError::ValidationFailed(
            "No --set arguments provided. Usage: acrm edit \"name\" --set key=value".to_string(),
        ));
    }

    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let mut cf = find_contact(contacts, name)?;

    let mut updated_fields = Vec::new();

    for set_arg in sets {
        let (key, value) = set_arg
            .split_once('=')
            .ok_or_else(|| {
                OpsError::ValidationFailed(format!(
                    "Invalid --set format '{set_arg}', expected key=value"
                ))
            })?;

        let key = key.trim();
        let value = value.trim();

        if ARRAY_FIELDS.contains(&key) {
            let values: Vec<String> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cf.raw_frontmatter =
                frontmatter::update_array_field(&cf.raw_frontmatter, key, &values);
        } else {
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
        .map_err(|e| OpsError::Internal(format!("Updated frontmatter produced invalid YAML: {e}")))?;

    // Validate before writing
    let errors = validation::validate_contact(&updated_contact);
    if !errors.is_empty() {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        return Err(OpsError::ValidationFailed(messages.join("; ")));
    }

    cf.contact = updated_contact;

    // Write directly to the existing path (preserving filename)
    let content = store::serialize_contact_file(&cf).map_err(internal)?;
    std::fs::write(&cf.path, &content)?;

    Ok(EditResult {
        name: cf.contact.name.clone(),
        updated_fields,
        path: cf.path.display().to_string(),
    })
}

/// Log an interaction with a contact.
pub fn log_interaction(
    root: &Path,
    name: &str,
    interaction_type: &str,
    summary: &str,
    notes: Option<&str>,
) -> Result<LogResult, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let mut cf = find_contact(contacts, name)?;
    let today = Local::now().date_naive();

    // Build the interaction entry
    let mut entry = format!("\n### {today} | {interaction_type} | {summary}\n");
    if let Some(n) = notes {
        entry.push('\n');
        entry.push_str(n);
        entry.push('\n');
    }

    // Insert after "## Interaction Log" heading
    if let Some(pos) = cf.body.find("## Interaction Log") {
        let insert_at = cf.body[pos..]
            .find('\n')
            .map(|i| pos + i + 1)
            .unwrap_or(cf.body.len());
        cf.body.insert_str(insert_at, &entry);
    } else {
        cf.body.push_str("\n## Interaction Log\n");
        cf.body.push_str(&entry);
    }

    // Update last_contacted via raw frontmatter editor
    cf.raw_frontmatter =
        frontmatter::update_field(&cf.raw_frontmatter, "last_contacted", &today.to_string());

    // Calculate and update next_follow_up if cadence is set
    let next_fu = if !cf.contact.follow_up_cadence.is_empty() {
        let next = next_follow_up(today, &cf.contact.follow_up_cadence)?;
        if let Some(date) = next {
            cf.raw_frontmatter = frontmatter::update_field(
                &cf.raw_frontmatter,
                "next_follow_up",
                &date.to_string(),
            );
        }
        next.map(|d| d.to_string())
    } else {
        None
    };

    // Write directly to existing file path (preserves comments via raw frontmatter)
    let content = store::serialize_contact_file(&cf).map_err(internal)?;
    std::fs::write(&cf.path, &content)?;

    Ok(LogResult {
        name: cf.contact.name.clone(),
        interaction_type: interaction_type.to_string(),
        summary: summary.to_string(),
        path: cf.path.display().to_string(),
        last_contacted: today.to_string(),
        next_follow_up: next_fu,
    })
}

/// Get contacts due for follow-up.
pub fn due(root: &Path) -> Result<Vec<DueContact>, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let today = Local::now().date_naive();

    let mut due_contacts: Vec<_> = contacts
        .into_iter()
        .filter(|cf| {
            cf.contact
                .next_follow_up
                .is_some_and(|d| d <= today)
        })
        .collect();

    due_contacts.sort_by_key(|cf| cf.contact.next_follow_up);

    let results = due_contacts
        .iter()
        .map(|cf| {
            let c = &cf.contact;
            let follow_up = c.next_follow_up.unwrap();
            DueContact {
                name: c.name.clone(),
                next_follow_up: follow_up.to_string(),
                last_contacted: c
                    .last_contacted
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "never".to_string()),
                overdue_days: (today - follow_up).num_days(),
            }
        })
        .collect();

    Ok(results)
}

/// Find a contact that would be deleted (first phase of two-phase delete).
pub fn find_delete_target(root: &Path, name: &str) -> Result<DeleteTarget, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let cf = find_contact(contacts, name)?;

    Ok(DeleteTarget {
        name: cf.contact.name,
        path: cf.path.display().to_string(),
    })
}

/// Actually delete a contact file (second phase of two-phase delete).
pub fn confirm_delete(root: &Path, name: &str) -> Result<DeleteResult, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let cf = find_contact(contacts, name)?;

    std::fs::remove_file(&cf.path)?;

    Ok(DeleteResult {
        name: cf.contact.name,
        path: cf.path.display().to_string(),
        deleted: true,
    })
}

/// Archive a contact (set status to archived, move to archive/).
pub fn archive(root: &Path, name: &str) -> Result<ArchiveResult, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    let mut cf = find_contact(contacts, name)?;

    // Update status to archived
    cf.raw_frontmatter = frontmatter::update_field(&cf.raw_frontmatter, "status", "archived");

    // Ensure archive directory exists
    let archive_dir = root.join("archive");
    std::fs::create_dir_all(&archive_dir)?;

    // Write to archive/
    let filename = cf
        .path
        .file_name()
        .ok_or_else(|| OpsError::Internal("Contact file has no filename".to_string()))?
        .to_owned();
    let archive_path = archive_dir.join(&filename);

    let content = store::serialize_contact_file(&cf).map_err(internal)?;
    std::fs::write(&archive_path, &content)?;

    // Remove original from contacts/
    std::fs::remove_file(&cf.path)?;

    Ok(ArchiveResult {
        name: cf.contact.name,
        action: "Archived".to_string(),
        from_path: cf.path.display().to_string(),
        to_path: archive_path.display().to_string(),
    })
}

/// Unarchive a contact (set status to active, move back to contacts/).
pub fn unarchive(root: &Path, name: &str) -> Result<ArchiveResult, OpsError> {
    let archive_dir = root.join("archive");
    let contacts = store::load_contacts_from_dir(&archive_dir).map_err(internal)?;
    let mut cf = find_contact(contacts, name)?;

    // Update status to active
    cf.raw_frontmatter = frontmatter::update_field(&cf.raw_frontmatter, "status", "active");

    // Write to contacts/
    let filename = cf
        .path
        .file_name()
        .ok_or_else(|| OpsError::Internal("Contact file has no filename".to_string()))?
        .to_owned();
    let contacts_path = root.join("contacts").join(&filename);

    let content = store::serialize_contact_file(&cf).map_err(internal)?;
    std::fs::write(&contacts_path, &content)?;

    // Remove from archive/
    std::fs::remove_file(&cf.path)?;

    Ok(ArchiveResult {
        name: cf.contact.name,
        action: "Unarchived".to_string(),
        from_path: cf.path.display().to_string(),
        to_path: contacts_path.display().to_string(),
    })
}

// ── Bulk result structs ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BulkResult {
    pub matched: usize,
    pub affected: usize,
    pub changes: Vec<BulkChange>,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct BulkChange {
    pub name: String,
    pub path: String,
    pub action: String,
}

// ── Bulk ops functions ──────────────────────────────────────────────────────

/// Query contacts using a parsed Query, returning all matching ContactFiles.
pub fn query(root: &Path, q: &crate::query::Query) -> Result<Vec<ContactFile>, OpsError> {
    let mut contacts = store::load_all_contacts(root).map_err(internal)?;
    contacts.retain(|cf| q.matches(&cf.contact));
    contacts.sort_by(|a, b| a.contact.name.cmp(&b.contact.name));
    Ok(contacts)
}

/// Apply field edits to all matched contacts.
///
/// Re-loads each file individually for fresh raw_frontmatter.
/// If dry_run is true, reports changes without writing.
pub fn bulk_update(
    root: &Path,
    matched: &[ContactFile],
    sets: &[String],
    dry_run: bool,
) -> Result<BulkResult, OpsError> {
    if sets.is_empty() {
        return Err(OpsError::ValidationFailed(
            "No --set arguments provided for bulk update".to_string(),
        ));
    }

    let mut changes = Vec::new();

    for cf in matched {
        // Re-load for fresh raw_frontmatter
        let mut fresh =
            store::parse_contact_file(&cf.path).map_err(|e| OpsError::Internal(e.to_string()))?;

        let mut action_parts = Vec::new();

        for set_arg in sets {
            let (key, value) = set_arg.split_once('=').ok_or_else(|| {
                OpsError::ValidationFailed(format!(
                    "Invalid --set format '{set_arg}', expected key=value"
                ))
            })?;
            let key = key.trim();
            let value = value.trim();

            if ARRAY_FIELDS.contains(&key) {
                let values: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                fresh.raw_frontmatter =
                    frontmatter::update_array_field(&fresh.raw_frontmatter, key, &values);
            } else {
                let yaml_value = if needs_quoting(value) {
                    format!("\"{}\"", value.replace('"', "\\\""))
                } else {
                    value.to_string()
                };
                fresh.raw_frontmatter =
                    frontmatter::update_field(&fresh.raw_frontmatter, key, &yaml_value);
            }

            action_parts.push(format!("set {key}={value}"));
        }

        // Re-parse and validate
        let updated_contact: Contact =
            serde_yaml::from_str(&fresh.raw_frontmatter).map_err(|e| {
                OpsError::Internal(format!("Updated frontmatter produced invalid YAML: {e}"))
            })?;
        let errors = validation::validate_contact(&updated_contact);
        if !errors.is_empty() {
            let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            return Err(OpsError::ValidationFailed(messages.join("; ")));
        }

        fresh.contact = updated_contact;

        if !dry_run {
            let content = store::serialize_contact_file(&fresh).map_err(internal)?;
            std::fs::write(&fresh.path, &content)?;
        }

        changes.push(BulkChange {
            name: fresh.contact.name.clone(),
            path: fresh.path.display().to_string(),
            action: action_parts.join(", "),
        });
    }

    Ok(BulkResult {
        matched: matched.len(),
        affected: changes.len(),
        changes,
        dry_run,
    })
}

/// Delete all matched contact files.
///
/// If dry_run is true, reports what would be deleted without removing files.
pub fn bulk_delete(
    _root: &Path,  // kept for API consistency with other bulk ops
    matched: &[ContactFile],
    dry_run: bool,
) -> Result<BulkResult, OpsError> {
    let mut changes = Vec::new();

    for cf in matched {
        if !dry_run {
            std::fs::remove_file(&cf.path)?;
        }

        changes.push(BulkChange {
            name: cf.contact.name.clone(),
            path: cf.path.display().to_string(),
            action: "deleted".to_string(),
        });
    }

    Ok(BulkResult {
        matched: matched.len(),
        affected: changes.len(),
        changes,
        dry_run,
    })
}

/// Archive all matched contacts (set status=archived, move to archive/).
///
/// If dry_run is true, reports what would be archived without moving files.
pub fn bulk_archive(
    root: &Path,
    matched: &[ContactFile],
    dry_run: bool,
) -> Result<BulkResult, OpsError> {
    let archive_dir = root.join("archive");
    if !dry_run {
        std::fs::create_dir_all(&archive_dir)?;
    }

    let mut changes = Vec::new();

    for cf in matched {
        // Re-load for fresh raw_frontmatter
        let mut fresh =
            store::parse_contact_file(&cf.path).map_err(|e| OpsError::Internal(e.to_string()))?;

        fresh.raw_frontmatter =
            frontmatter::update_field(&fresh.raw_frontmatter, "status", "archived");

        // Re-parse so the Contact struct reflects the status change
        let updated_contact: Contact =
            serde_yaml::from_str(&fresh.raw_frontmatter).map_err(|e| {
                OpsError::Internal(format!("Updated frontmatter produced invalid YAML: {e}"))
            })?;
        fresh.contact = updated_contact;

        if !dry_run {
            let filename = cf
                .path
                .file_name()
                .ok_or_else(|| OpsError::Internal("Contact file has no filename".to_string()))?;
            let archive_path = archive_dir.join(filename);

            let content = store::serialize_contact_file(&fresh).map_err(internal)?;
            std::fs::write(&archive_path, &content)?;
            std::fs::remove_file(&cf.path)?;
        }

        changes.push(BulkChange {
            name: cf.contact.name.clone(),
            path: cf.path.display().to_string(),
            action: "archived".to_string(),
        });
    }

    Ok(BulkResult {
        matched: matched.len(),
        affected: changes.len(),
        changes,
        dry_run,
    })
}

/// Add/remove tags on all matched contacts.
///
/// If dry_run is true, reports tag changes without writing.
pub fn bulk_tag(
    _root: &Path,  // kept for API consistency with other bulk ops
    matched: &[ContactFile],
    add_tags: &[String],
    remove_tags: &[String],
    dry_run: bool,
) -> Result<BulkResult, OpsError> {
    let mut changes = Vec::new();

    for cf in matched {
        // Re-load for fresh raw_frontmatter
        let mut fresh =
            store::parse_contact_file(&cf.path).map_err(|e| OpsError::Internal(e.to_string()))?;

        let mut current_tags = fresh.contact.tags.clone();
        let mut action_parts = Vec::new();

        // Add tags (deduplicate)
        for tag in add_tags {
            if !current_tags.contains(tag) {
                current_tags.push(tag.clone());
                action_parts.push(format!("added tag {tag}"));
            }
        }

        // Remove tags
        for tag in remove_tags {
            if current_tags.contains(tag) {
                current_tags.retain(|t| t != tag);
                action_parts.push(format!("removed tag {tag}"));
            }
        }

        if action_parts.is_empty() {
            continue; // No changes needed for this contact
        }

        fresh.raw_frontmatter =
            frontmatter::update_array_field(&fresh.raw_frontmatter, "tags", &current_tags);

        // Re-parse
        let updated_contact: Contact =
            serde_yaml::from_str(&fresh.raw_frontmatter).map_err(|e| {
                OpsError::Internal(format!("Updated frontmatter produced invalid YAML: {e}"))
            })?;
        fresh.contact = updated_contact;

        if !dry_run {
            let content = store::serialize_contact_file(&fresh).map_err(internal)?;
            std::fs::write(&fresh.path, &content)?;
        }

        changes.push(BulkChange {
            name: cf.contact.name.clone(),
            path: cf.path.display().to_string(),
            action: action_parts.join(", "),
        });
    }

    Ok(BulkResult {
        matched: matched.len(),
        affected: changes.len(),
        changes,
        dry_run,
    })
}

// ── Utility functions ───────────────────────────────────────────────────────

/// Calculate the next follow-up date from a given date and cadence string.
/// Returns Ok(None) if cadence is empty.
/// Returns Err if cadence is unknown.
pub fn next_follow_up(from_date: NaiveDate, cadence: &str) -> Result<Option<NaiveDate>, OpsError> {
    let cadence = cadence.trim();
    if cadence.is_empty() {
        return Ok(None);
    }

    let next = match cadence {
        "weekly" => from_date + Duration::days(7),
        "biweekly" | "bi-weekly" => from_date + Duration::days(14),
        "monthly" => from_date
            .checked_add_months(Months::new(1))
            .ok_or_else(|| OpsError::Internal("Date overflow adding 1 month".to_string()))?,
        "quarterly" => from_date
            .checked_add_months(Months::new(3))
            .ok_or_else(|| OpsError::Internal("Date overflow adding 3 months".to_string()))?,
        "yearly" | "annually" => from_date
            .checked_add_months(Months::new(12))
            .ok_or_else(|| OpsError::Internal("Date overflow adding 12 months".to_string()))?,
        _ => {
            return Err(OpsError::Internal(format!(
                "Unknown cadence: '{cadence}'. Supported: weekly, biweekly, monthly, quarterly, yearly"
            )));
        }
    };

    Ok(Some(next))
}

/// Determine if a value needs YAML quoting.
/// Values that are plain YAML scalars (numbers, booleans, dates, known enums) don't need quotes.
pub fn needs_quoting(value: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn test_next_follow_up_weekly() {
        let result = next_follow_up(date(2026, 3, 5), "weekly").unwrap();
        assert_eq!(result, Some(date(2026, 3, 12)));
    }

    #[test]
    fn test_next_follow_up_biweekly() {
        let result = next_follow_up(date(2026, 3, 5), "biweekly").unwrap();
        assert_eq!(result, Some(date(2026, 3, 19)));
    }

    #[test]
    fn test_next_follow_up_bi_weekly_hyphenated() {
        let result = next_follow_up(date(2026, 3, 5), "bi-weekly").unwrap();
        assert_eq!(result, Some(date(2026, 3, 19)));
    }

    #[test]
    fn test_next_follow_up_monthly() {
        let result = next_follow_up(date(2026, 3, 5), "monthly").unwrap();
        assert_eq!(result, Some(date(2026, 4, 5)));
    }

    #[test]
    fn test_next_follow_up_quarterly() {
        let result = next_follow_up(date(2026, 3, 5), "quarterly").unwrap();
        assert_eq!(result, Some(date(2026, 6, 5)));
    }

    #[test]
    fn test_next_follow_up_yearly() {
        let result = next_follow_up(date(2026, 3, 5), "yearly").unwrap();
        assert_eq!(result, Some(date(2027, 3, 5)));
    }

    #[test]
    fn test_next_follow_up_annually() {
        let result = next_follow_up(date(2026, 3, 5), "annually").unwrap();
        assert_eq!(result, Some(date(2027, 3, 5)));
    }

    #[test]
    fn test_next_follow_up_empty_cadence() {
        let result = next_follow_up(date(2026, 3, 5), "").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_next_follow_up_whitespace_cadence() {
        let result = next_follow_up(date(2026, 3, 5), "  ").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_next_follow_up_unknown_cadence() {
        let result = next_follow_up(date(2026, 3, 5), "daily");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown cadence"));
        assert!(err.contains("daily"));
        assert!(err.contains("weekly"));
        assert!(err.contains("monthly"));
    }
}

#[cfg(test)]
mod bulk_tests {
    use super::*;
    use crate::query::Query;
    use std::fs;
    use tempfile::TempDir;

    /// Set up a temp CRM root with contacts/ dir, template, and sample contacts.
    fn setup_bulk_test() -> (TempDir, std::path::PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        // Create contacts/ and templates/ directories
        fs::create_dir_all(root.join("contacts")).unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();

        // Create a minimal template (needed for some ops but not for parse)
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

        // Create two sample contacts
        let contact1 = r#"---
id: "aaa-111"
name: "Alice Test"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: "Acme"
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
  - "engineer"
status: dormant
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

        let contact2 = r#"---
id: "bbb-222"
name: "Bob Test"
aliases: []
pronouns: ""
email: []
phone: []
address: []
company: "Globex"
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
relationship: colleague
tags:
  - "python"
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: low
source: manual
source_id: ""
etag: ""
---

## Notes

## Interaction Log
"#;

        fs::write(root.join("contacts/alice-test.md"), contact1).unwrap();
        fs::write(root.join("contacts/bob-test.md"), contact2).unwrap();

        (tmp, root)
    }

    #[test]
    fn bulk_query_filters_by_status() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let results = query(&root, &q).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].contact.name, "Alice Test");
    }

    #[test]
    fn bulk_query_returns_sorted_by_name() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("source=manual").unwrap();
        let results = query(&root, &q).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].contact.name, "Alice Test");
        assert_eq!(results[1].contact.name, "Bob Test");
    }

    #[test]
    fn bulk_update_applies_changes() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result =
            bulk_update(&root, &matched, &["status=active".to_string()], false).unwrap();
        assert_eq!(result.matched, 1);
        assert_eq!(result.affected, 1);
        assert!(!result.dry_run);
        assert!(result.changes[0].action.contains("set status=active"));

        // Verify file was actually changed
        let content = fs::read_to_string(root.join("contacts/alice-test.md")).unwrap();
        assert!(content.contains("status: active"));
    }

    #[test]
    fn bulk_update_dry_run_does_not_write() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result =
            bulk_update(&root, &matched, &["status=active".to_string()], true).unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.dry_run);

        // File should NOT have changed
        let content = fs::read_to_string(root.join("contacts/alice-test.md")).unwrap();
        assert!(content.contains("status: dormant"));
    }

    #[test]
    fn bulk_delete_removes_files() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_delete(&root, &matched, false).unwrap();
        assert_eq!(result.affected, 1);
        assert!(!root.join("contacts/alice-test.md").exists());
        // Bob should still exist
        assert!(root.join("contacts/bob-test.md").exists());
    }

    #[test]
    fn bulk_delete_dry_run_preserves_files() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_delete(&root, &matched, true).unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.dry_run);
        // File should still exist
        assert!(root.join("contacts/alice-test.md").exists());
    }

    #[test]
    fn bulk_archive_moves_and_updates_status() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_archive(&root, &matched, false).unwrap();
        assert_eq!(result.affected, 1);

        // Original should be gone
        assert!(!root.join("contacts/alice-test.md").exists());
        // Archive file should exist
        let archive_path = root.join("archive/alice-test.md");
        assert!(archive_path.exists());
        // Status should be archived
        let content = fs::read_to_string(&archive_path).unwrap();
        assert!(content.contains("status: archived"));
    }

    #[test]
    fn bulk_archive_dry_run_preserves_files() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_archive(&root, &matched, true).unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.dry_run);
        // Original should still be there
        assert!(root.join("contacts/alice-test.md").exists());
        // Archive should not exist
        assert!(!root.join("archive/alice-test.md").exists());
    }

    #[test]
    fn bulk_tag_adds_tags() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_tag(
            &root,
            &matched,
            &["new-tag".to_string()],
            &[],
            false,
        )
        .unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.changes[0].action.contains("added tag new-tag"));

        // Verify file was changed
        let content = fs::read_to_string(root.join("contacts/alice-test.md")).unwrap();
        assert!(content.contains("new-tag"));
    }

    #[test]
    fn bulk_tag_removes_tags() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_tag(
            &root,
            &matched,
            &[],
            &["rust".to_string()],
            false,
        )
        .unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.changes[0].action.contains("removed tag rust"));
    }

    #[test]
    fn bulk_tag_dry_run_does_not_write() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        let result = bulk_tag(
            &root,
            &matched,
            &["new-tag".to_string()],
            &[],
            true,
        )
        .unwrap();
        assert_eq!(result.affected, 1);
        assert!(result.dry_run);

        // File should NOT have changed
        let content = fs::read_to_string(root.join("contacts/alice-test.md")).unwrap();
        assert!(!content.contains("new-tag"));
    }

    #[test]
    fn bulk_tag_deduplicates_existing_tags() {
        let (_tmp, root) = setup_bulk_test();
        let q = Query::parse("status=dormant").unwrap();
        let matched = query(&root, &q).unwrap();

        // Try adding "rust" which already exists
        let result = bulk_tag(
            &root,
            &matched,
            &["rust".to_string()],
            &[],
            false,
        )
        .unwrap();
        // Should have 0 affected since no changes were needed
        assert_eq!(result.affected, 0);
    }
}
