use std::path::Path;

use serde::Serialize;
use url::Url;

use super::OpsError;
use crate::frontmatter;
use crate::models::ContactFile;
use crate::store;
use crate::sync::{
    carddav::CardDavClient, dedup, filter::SyncFilter, push, vcard_map, vcard_write,
};

// ── Credential and option structs ────────────────────────────────────────────

/// Credentials for iCloud CardDAV sync. Ops never loads these from keyring;
/// the caller (CLI or MCP) constructs this from its own credential source.
pub struct SyncCredentials {
    pub apple_id: String,
    pub app_password: String,
}

/// Options controlling sync behavior.
pub struct SyncOpts {
    pub force: bool,
    pub dry_run: bool,
}

// ── Result structs ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub new: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub dry_run: bool,
    pub contacts: Vec<SyncedContact>,
}

#[derive(Debug, Serialize)]
pub struct SyncedContact {
    pub name: String,
    pub action: String, // "new", "updated", "unchanged"
}

#[derive(Debug, Serialize)]
pub struct PushSyncResult {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub conflicted: usize,
    pub failed: usize,
    pub dry_run: bool,
    pub details: Vec<PushSyncDetail>,
}

#[derive(Debug, Serialize)]
pub struct PushSyncDetail {
    pub name: String,
    pub action: String,
    pub error: Option<String>,
}

// ── Sync operations ──────────────────────────────────────────────────────────

/// Pull contacts from iCloud via CardDAV.
///
/// Discovers the address book, fetches all vCards, and creates/updates local
/// contact files. Progress messages are NOT emitted here (presentation stays
/// in the CLI wrapper).
pub fn sync_pull(
    root: &Path,
    credentials: &SyncCredentials,
    filter: &SyncFilter,
    opts: &SyncOpts,
) -> Result<SyncResult, OpsError> {
    let client = CardDavClient::new(&credentials.apple_id, &credentials.app_password)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let addressbook_url = client
        .discover_address_book()
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let entries = client
        .fetch_vcard_list(&addressbook_url)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let existing_contacts = store::load_all_contacts(root).map_err(internal)?;

    let mut new_count: usize = 0;
    let mut updated_count: usize = 0;
    let mut unchanged_count: usize = 0;
    let mut synced: Vec<SyncedContact> = Vec::new();

    for entry in &entries {
        let vcard_url = resolve_vcard_url(&addressbook_url, &entry.href)
            .map_err(|e| OpsError::SyncError(e.to_string()))?;

        let vcard_text = match client.fetch_vcard(&vcard_url) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Warning: failed to fetch {}: {}", entry.href, e);
                continue;
            }
        };

        let uid = extract_uid_from_href(&entry.href);

        // Cache raw vCard text for round-trip preservation during push
        if !opts.dry_run {
            if let Err(e) = vcard_write::write_cached_vcard(root, &uid, &vcard_text) {
                eprintln!("Warning: failed to cache vCard {}: {}", uid, e);
            }
        }

        let mapped = match vcard_map::map_vcard_to_contact(&vcard_text, &uid, &entry.etag) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Warning: failed to parse vCard {}: {}", entry.href, e);
                continue;
            }
        };

        let contact_name = mapped.contact.name.clone();

        if let Some(existing) = dedup::find_existing_by_source_id(&existing_contacts, &uid) {
            if !opts.force && !dedup::should_update(existing, &entry.etag) {
                if !opts.dry_run {
                    let _ =
                        vcard_write::cache_contact_snapshot(root, &uid, &mapped.contact);
                }
                unchanged_count += 1;
                synced.push(SyncedContact {
                    name: contact_name,
                    action: "unchanged".to_string(),
                });
                continue;
            }

            // Apply pull filter to existing contacts (skip non-matching updates)
            if !filter.is_empty() && !filter.matches(&mapped.contact) {
                unchanged_count += 1;
                synced.push(SyncedContact {
                    name: contact_name,
                    action: "unchanged".to_string(),
                });
                continue;
            }

            // Update existing contact
            if !opts.dry_run {
                update_existing_contact(existing, &mapped, &entry.etag)
                    .map_err(|e| OpsError::SyncError(e.to_string()))?;
                if let Err(e) =
                    vcard_write::cache_contact_snapshot(root, &uid, &mapped.contact)
                {
                    eprintln!(
                        "Warning: failed to cache contact snapshot for {}: {}",
                        contact_name, e
                    );
                }
            }
            updated_count += 1;
            synced.push(SyncedContact {
                name: contact_name,
                action: "updated".to_string(),
            });
        } else {
            // New contact
            if !opts.dry_run {
                create_new_contact(root, &mapped, &uid, &entry.etag)
                    .map_err(|e| OpsError::SyncError(e.to_string()))?;
                if let Err(e) =
                    vcard_write::cache_contact_snapshot(root, &uid, &mapped.contact)
                {
                    eprintln!(
                        "Warning: failed to cache contact snapshot for {}: {}",
                        contact_name, e
                    );
                }
            }
            new_count += 1;
            synced.push(SyncedContact {
                name: contact_name,
                action: "new".to_string(),
            });
        }
    }

    Ok(SyncResult {
        new: new_count,
        updated: updated_count,
        unchanged: unchanged_count,
        dry_run: opts.dry_run,
        contacts: synced,
    })
}

/// Push local changes to iCloud via CardDAV.
pub fn sync_push(
    root: &Path,
    credentials: &SyncCredentials,
    filter: &SyncFilter,
    opts: &SyncOpts,
) -> Result<PushSyncResult, OpsError> {
    let client = CardDavClient::new(&credentials.apple_id, &credentials.app_password)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let addressbook_url = client
        .discover_address_book()
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let server_entries = client
        .fetch_vcard_list(&addressbook_url)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let mut contacts = store::load_all_contacts(root).map_err(internal)?;

    // Apply push filter to active contacts before changeset computation.
    if !filter.is_empty() {
        contacts.retain(|cf| filter.matches(&cf.contact));
    }

    let changeset = push::compute_push_changeset(root, contacts, &server_entries)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    if opts.dry_run {
        let mut details = Vec::new();
        for cf in &changeset.creates {
            details.push(PushSyncDetail {
                name: cf.contact.name.clone(),
                action: "would_create".to_string(),
                error: None,
            });
        }
        for (cf, _) in &changeset.updates {
            details.push(PushSyncDetail {
                name: cf.contact.name.clone(),
                action: "would_update".to_string(),
                error: None,
            });
        }
        for (_, _, name) in &changeset.deletes {
            details.push(PushSyncDetail {
                name: name.clone(),
                action: "would_delete".to_string(),
                error: None,
            });
        }
        for (cf, local_etag, server_etag) in &changeset.conflicts {
            details.push(PushSyncDetail {
                name: cf.contact.name.clone(),
                action: "conflict".to_string(),
                error: Some(format!(
                    "local etag: {}, server etag: {}",
                    local_etag, server_etag
                )),
            });
        }

        return Ok(PushSyncResult {
            created: changeset.creates.len(),
            updated: changeset.updates.len(),
            deleted: changeset.deletes.len(),
            conflicted: changeset.conflicts.len(),
            failed: 0,
            dry_run: true,
            details,
        });
    }

    let push_result =
        push::execute_push(&client, &addressbook_url, root, &changeset, opts.force)
            .map_err(|e| OpsError::SyncError(e.to_string()))?;

    Ok(PushSyncResult {
        created: push_result.created,
        updated: push_result.updated,
        deleted: push_result.deleted,
        conflicted: push_result.conflicted,
        failed: push_result.failed,
        dry_run: false,
        details: push_result
            .details
            .into_iter()
            .map(|d| PushSyncDetail {
                name: d.name,
                action: d.action,
                error: d.error,
            })
            .collect(),
    })
}

/// Bidirectional sync: pull then push.
pub fn sync_bidirectional(
    root: &Path,
    credentials: &SyncCredentials,
    pull_filter: &SyncFilter,
    push_filter: &SyncFilter,
    opts: &SyncOpts,
) -> Result<(SyncResult, PushSyncResult), OpsError> {
    let pull_result = sync_pull(root, credentials, pull_filter, opts)?;
    let push_result = sync_push(root, credentials, push_filter, opts)?;
    Ok((pull_result, push_result))
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Map an anyhow::Error to OpsError::Internal
fn internal(e: anyhow::Error) -> OpsError {
    OpsError::Internal(e.to_string())
}

/// Create a new contact file from a mapped vCard.
fn create_new_contact(
    crm_root: &Path,
    mapped: &vcard_map::MappedContact,
    uid: &str,
    etag: &str,
) -> anyhow::Result<()> {
    let mut fm = store::generate_raw_frontmatter(&mapped.contact, crm_root)?;

    if !mapped.contact.email.is_empty() {
        fm = frontmatter::update_array_field(&fm, "email", &mapped.contact.email);
    }
    if !mapped.contact.phone.is_empty() {
        fm = frontmatter::update_array_field(&fm, "phone", &mapped.contact.phone);
    }
    if !mapped.contact.tags.is_empty() {
        fm = frontmatter::update_array_field(&fm, "tags", &mapped.contact.tags);
    }

    fm = frontmatter::update_field(&fm, "source", "icloud");
    fm = frontmatter::update_field(&fm, "source_id", &format!("\"{}\"", uid));
    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", etag));
    // iCloud imports default to low priority (vs. medium for manually `add`ed
    // contacts) -- most of an address book is unfiltered, not deliberately
    // curated relationships.
    fm = frontmatter::update_field(&fm, "priority", "low");

    if !mapped.contact.website.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "website",
            &format!("\"{}\"", mapped.contact.website),
        );
    }
    if let Some(bday) = mapped.contact.birthday {
        fm = frontmatter::update_field(&fm, "birthday", &bday.to_string());
    }

    if !fm.ends_with('\n') {
        fm.push('\n');
    }

    let mut body = String::from("## Notes\n\n");
    if !mapped.notes.is_empty() {
        body.push_str(&mapped.notes);
        body.push_str("\n\n");
    }
    body.push_str("## Interaction Log\n");

    let cf = ContactFile {
        contact: mapped.contact.clone(),
        body,
        path: crm_root.join("contacts"),
        raw_frontmatter: fm,
    };

    store::write_contact(crm_root, &cf)?;
    Ok(())
}

/// Update an existing contact's frontmatter fields from a new vCard version.
/// Routes through store::serialize_contact_file for consistent serialization.
fn update_existing_contact(
    existing: &ContactFile,
    mapped: &vcard_map::MappedContact,
    new_etag: &str,
) -> anyhow::Result<()> {
    let mut fm = existing.raw_frontmatter.clone();

    fm = frontmatter::update_field(&fm, "name", &format!("\"{}\"", mapped.contact.name));
    fm = frontmatter::update_array_field(&fm, "email", &mapped.contact.email);
    fm = frontmatter::update_array_field(&fm, "phone", &mapped.contact.phone);

    if !mapped.contact.company.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "company",
            &format!("\"{}\"", mapped.contact.company),
        );
    }
    if !mapped.contact.role.is_empty() {
        fm = frontmatter::update_field(&fm, "role", &format!("\"{}\"", mapped.contact.role));
    }
    if !mapped.contact.website.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "website",
            &format!("\"{}\"", mapped.contact.website),
        );
    }
    if let Some(bday) = mapped.contact.birthday {
        fm = frontmatter::update_field(&fm, "birthday", &bday.to_string());
    }
    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", new_etag));

    if !fm.ends_with('\n') {
        fm.push('\n');
    }

    // Use serialize_contact_file instead of manual format! to fix tech debt
    let updated_cf = ContactFile {
        contact: existing.contact.clone(),
        body: existing.body.clone(),
        path: existing.path.clone(),
        raw_frontmatter: fm,
    };
    let content = store::serialize_contact_file(&updated_cf)
        .map_err(|e| anyhow::anyhow!("Failed to serialize contact: {}", e))?;
    std::fs::write(&existing.path, content)
        .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", existing.path.display(), e))?;

    Ok(())
}

/// Resolve a vCard href to a full URL relative to the addressbook URL.
fn resolve_vcard_url(addressbook_url: &Url, href: &str) -> anyhow::Result<Url> {
    if href.starts_with('/') {
        let base = format!(
            "{}://{}",
            addressbook_url.scheme(),
            addressbook_url
                .host_str()
                .unwrap_or("contacts.icloud.com")
        );
        Url::parse(&format!("{}{}", base, href))
            .map_err(|e| anyhow::anyhow!("Failed to resolve vCard URL: {}", e))
    } else {
        addressbook_url
            .join(href)
            .map_err(|e| anyhow::anyhow!("Failed to join vCard URL: {}", e))
    }
}

/// Extract a UID from a vCard href path.
/// e.g., "/123/carddavhome/card/ABC-DEF-123.vcf" -> "ABC-DEF-123"
pub fn extract_uid_from_href(href: &str) -> String {
    href.rsplit('/')
        .next()
        .unwrap_or(href)
        .trim_end_matches(".vcf")
        .to_string()
}
