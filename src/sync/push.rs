use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use url::Url;

use crate::frontmatter;
use crate::models::{ContactFile, Status};
use crate::store;
use crate::sync::carddav::{CardDavClient, VCardEntry};
use crate::sync::vcard_write;

/// A single action detail from a push operation.
#[derive(Serialize)]
pub struct PushDetail {
    pub name: String,
    pub action: String, // "created", "updated", "deleted", "conflict", "failed"
    pub error: Option<String>,
}

/// Result of executing a push changeset.
#[derive(Serialize)]
pub struct PushResult {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub conflicted: usize,
    pub failed: usize,
    pub details: Vec<PushDetail>,
}

impl fmt::Display for PushResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Push complete: {} created, {} updated, {} deleted, {} conflicts",
            self.created, self.updated, self.deleted, self.conflicted
        )?;
        if self.failed > 0 {
            write!(f, ", {} failed", self.failed)?;
        }
        Ok(())
    }
}

/// A computed set of changes to push to the server.
pub struct PushChangeset {
    /// Contacts with no icloud source -- need to be created on the server
    pub creates: Vec<ContactFile>,
    /// Contacts with source "icloud" whose vCard differs from cache -- (contact, cached_vcard)
    pub updates: Vec<(ContactFile, String)>,
    /// Archived contacts with source "icloud" -- (source_id, etag, name)
    pub deletes: Vec<(String, String, String)>,
    /// Contacts whose local etag differs from server etag -- (contact, local_etag, server_etag)
    pub conflicts: Vec<(ContactFile, String, String)>,
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

/// Compute the push changeset by comparing local contacts against server state and vCard cache.
///
/// For each contact:
/// - source != "icloud" (or empty) -> create
/// - source == "icloud" with source_id, server etag differs -> conflict
/// - source == "icloud" with source_id, vCard differs from cache -> update
/// - source == "icloud" with source_id, vCard matches cache -> skip (unchanged)
/// - source == "icloud" with no source_id -> skip
/// - Archived contacts with source == "icloud" and source_id -> delete
pub fn compute_push_changeset(
    crm_root: &Path,
    contacts: Vec<ContactFile>,
    server_entries: &[VCardEntry],
) -> Result<PushChangeset> {
    // Build a lookup of server etags by source_id
    let mut server_etags: HashMap<String, String> = HashMap::new();
    for entry in server_entries {
        let uid = extract_uid_from_href(&entry.href);
        server_etags.insert(uid, entry.etag.clone());
    }

    let mut creates = Vec::new();
    let mut updates = Vec::new();
    let mut deletes = Vec::new();
    let mut conflicts = Vec::new();

    for cf in contacts {
        // Skip non-icloud contacts -> they are new (need to create on server)
        if cf.contact.source != "icloud" {
            creates.push(cf);
            continue;
        }

        // icloud contact with no source_id -> skip (incomplete sync state)
        if cf.contact.source_id.is_empty() {
            continue;
        }

        let source_id = cf.contact.source_id.clone();

        // Check if this contact is archived -> delete from server
        if matches!(cf.contact.status, Some(Status::Archived)) {
            deletes.push((
                source_id.clone(),
                cf.contact.etag.clone(),
                cf.contact.name.clone(),
            ));
            continue;
        }

        // Check server etag vs local etag for conflict detection
        if let Some(server_etag) = server_etags.get(&source_id) {
            if server_etag != &cf.contact.etag && !cf.contact.etag.is_empty() {
                let local_etag = cf.contact.etag.clone();
                let s_etag = server_etag.clone();
                conflicts.push((cf, local_etag, s_etag));
                continue;
            }
        }

        // Compare serialized vCard with cached version
        let cached = vcard_write::read_cached_vcard(crm_root, &source_id);
        let serialized = if let Some(ref cached_text) = cached {
            vcard_write::merge_contact_to_vcard(&cf.contact, cached_text)?
        } else {
            vcard_write::contact_to_vcard(&cf.contact)?
        };

        if let Some(ref cached_text) = cached {
            if serialized == *cached_text {
                // Unchanged -- skip
                continue;
            }
        }

        // Changed -- add to updates
        updates.push((cf, cached.unwrap_or_default()));
    }

    // Check archived contacts from archive/ directory
    let archive_dir = crm_root.join("archive");
    if archive_dir.is_dir() {
        let archived_contacts = store::load_contacts_from_dir(&archive_dir)?;
        for cf in archived_contacts {
            if cf.contact.source == "icloud" && !cf.contact.source_id.is_empty() {
                deletes.push((
                    cf.contact.source_id.clone(),
                    cf.contact.etag.clone(),
                    cf.contact.name.clone(),
                ));
            }
        }
    }

    Ok(PushChangeset {
        creates,
        updates,
        deletes,
        conflicts,
    })
}

/// Execute a push changeset against the CardDAV server.
///
/// For creates: serialize via contact_to_vcard, PUT with no etag, update frontmatter on success.
/// For updates: serialize via merge_contact_to_vcard, PUT with If-Match etag, update cache on success.
/// For deletes: DELETE with If-Match etag, remove cache on success.
/// For conflicts: if force=true, treat as updates. Otherwise skip and report.
pub fn execute_push(
    client: &CardDavClient,
    addressbook_url: &Url,
    crm_root: &Path,
    changeset: &PushChangeset,
    force: bool,
) -> Result<PushResult> {
    let mut result = PushResult {
        created: 0,
        updated: 0,
        deleted: 0,
        conflicted: 0,
        failed: 0,
        details: Vec::new(),
    };

    // --- Process creates ---
    for cf in &changeset.creates {
        let vcard_text = match vcard_write::contact_to_vcard(&cf.contact) {
            Ok(v) => v,
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("serialize error: {}", e)),
                });
                continue;
            }
        };

        let new_uuid = uuid::Uuid::new_v4().to_string();
        let url = match CardDavClient::build_vcard_url(addressbook_url, &new_uuid) {
            Ok(u) => u,
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("URL build error: {}", e)),
                });
                continue;
            }
        };

        match client.put_vcard(&url, &vcard_text, None) {
            Ok(returned_etag) => {
                let new_etag = if returned_etag.is_empty() {
                    // PROPFIND fallback: fetch list and find etag by UUID
                    match client.fetch_vcard_list(addressbook_url) {
                        Ok(entries) => {
                            entries.iter()
                                .find(|e| extract_uid_from_href(&e.href) == new_uuid)
                                .map(|e| e.etag.clone())
                                .unwrap_or_default()
                        }
                        Err(_) => String::new(),
                    }
                } else {
                    returned_etag
                };

                // Update frontmatter: set source, source_id, etag
                let mut fm = cf.raw_frontmatter.clone();
                fm = frontmatter::update_field(&fm, "source", "icloud");
                fm = frontmatter::update_field(&fm, "source_id", &format!("\"{}\"", new_uuid));
                fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", new_etag));
                if !fm.ends_with('\n') {
                    fm.push('\n');
                }
                let content = format!("---\n{}---\n\n{}", fm, cf.body);
                if let Err(e) = std::fs::write(&cf.path, content) {
                    eprintln!("Warning: failed to update frontmatter for {}: {}", cf.contact.name, e);
                }

                // Cache the vCard
                if let Err(e) = vcard_write::write_cached_vcard(crm_root, &new_uuid, &vcard_text) {
                    eprintln!("Warning: failed to cache vCard for {}: {}", cf.contact.name, e);
                }

                result.created += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "created".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("{}", e)),
                });
            }
        }
    }

    // --- Process updates ---
    for (cf, cached_vcard) in &changeset.updates {
        let vcard_text = match vcard_write::merge_contact_to_vcard(&cf.contact, cached_vcard) {
            Ok(v) => v,
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("serialize error: {}", e)),
                });
                continue;
            }
        };

        let url = match CardDavClient::build_vcard_url(addressbook_url, &cf.contact.source_id) {
            Ok(u) => u,
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("URL build error: {}", e)),
                });
                continue;
            }
        };

        match client.put_vcard(&url, &vcard_text, Some(&cf.contact.etag)) {
            Ok(returned_etag) => {
                let new_etag = if returned_etag.is_empty() {
                    match client.fetch_vcard_list(addressbook_url) {
                        Ok(entries) => {
                            entries.iter()
                                .find(|e| extract_uid_from_href(&e.href) == cf.contact.source_id)
                                .map(|e| e.etag.clone())
                                .unwrap_or_default()
                        }
                        Err(_) => String::new(),
                    }
                } else {
                    returned_etag
                };

                // Update frontmatter: etag only
                let mut fm = cf.raw_frontmatter.clone();
                fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", new_etag));
                if !fm.ends_with('\n') {
                    fm.push('\n');
                }
                let content = format!("---\n{}---\n\n{}", fm, cf.body);
                if let Err(e) = std::fs::write(&cf.path, content) {
                    eprintln!("Warning: failed to update frontmatter for {}: {}", cf.contact.name, e);
                }

                // Update cache
                if let Err(e) = vcard_write::write_cached_vcard(crm_root, &cf.contact.source_id, &vcard_text) {
                    eprintln!("Warning: failed to cache vCard for {}: {}", cf.contact.name, e);
                }

                result.updated += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "updated".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: cf.contact.name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("{}", e)),
                });
            }
        }
    }

    // --- Process deletes ---
    for (source_id, etag, name) in &changeset.deletes {
        let url = match CardDavClient::build_vcard_url(addressbook_url, source_id) {
            Ok(u) => u,
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("URL build error: {}", e)),
                });
                continue;
            }
        };

        match client.delete_vcard(&url, etag) {
            Ok(()) => {
                if let Err(e) = vcard_write::delete_cached_vcard(crm_root, source_id) {
                    eprintln!("Warning: failed to remove cached vCard for {}: {}", name, e);
                }

                result.deleted += 1;
                result.details.push(PushDetail {
                    name: name.clone(),
                    action: "deleted".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail {
                    name: name.clone(),
                    action: "failed".to_string(),
                    error: Some(format!("{}", e)),
                });
            }
        }
    }

    // --- Process conflicts ---
    for (cf, local_etag, server_etag) in &changeset.conflicts {
        if force {
            // Force: treat as update using server etag for If-Match
            let cached = vcard_write::read_cached_vcard(crm_root, &cf.contact.source_id);
            let vcard_text = match cached {
                Some(ref cached_text) => vcard_write::merge_contact_to_vcard(&cf.contact, cached_text)?,
                None => vcard_write::contact_to_vcard(&cf.contact)?,
            };

            let url = CardDavClient::build_vcard_url(addressbook_url, &cf.contact.source_id)?;

            match client.put_vcard(&url, &vcard_text, Some(server_etag)) {
                Ok(returned_etag) => {
                    let new_etag = if returned_etag.is_empty() {
                        match client.fetch_vcard_list(addressbook_url) {
                            Ok(entries) => {
                                entries.iter()
                                    .find(|e| extract_uid_from_href(&e.href) == cf.contact.source_id)
                                    .map(|e| e.etag.clone())
                                    .unwrap_or_default()
                            }
                            Err(_) => String::new(),
                        }
                    } else {
                        returned_etag
                    };

                    // Update frontmatter etag
                    let mut fm = cf.raw_frontmatter.clone();
                    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", new_etag));
                    if !fm.ends_with('\n') {
                        fm.push('\n');
                    }
                    let content = format!("---\n{}---\n\n{}", fm, cf.body);
                    if let Err(e) = std::fs::write(&cf.path, content) {
                        eprintln!("Warning: failed to update frontmatter for {}: {}", cf.contact.name, e);
                    }

                    if let Err(e) = vcard_write::write_cached_vcard(crm_root, &cf.contact.source_id, &vcard_text) {
                        eprintln!("Warning: failed to cache vCard for {}: {}", cf.contact.name, e);
                    }

                    result.updated += 1;
                    result.details.push(PushDetail {
                        name: cf.contact.name.clone(),
                        action: "updated".to_string(),
                        error: None,
                    });
                }
                Err(e) => {
                    result.failed += 1;
                    result.details.push(PushDetail {
                        name: cf.contact.name.clone(),
                        action: "failed".to_string(),
                        error: Some(format!("{}", e)),
                    });
                }
            }
        } else {
            // No force: skip and report conflict
            result.conflicted += 1;
            result.details.push(PushDetail {
                name: cf.contact.name.clone(),
                action: "conflict".to_string(),
                error: Some(format!(
                    "server ETag {} differs from local {}",
                    server_etag, local_etag
                )),
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Contact, Status};
    use crate::sync::vcard_write;
    use std::path::PathBuf;

    fn make_contact(
        name: &str,
        source: &str,
        source_id: &str,
        etag: &str,
        status: Option<Status>,
    ) -> Contact {
        Contact {
            id: format!("id-{}", name.to_lowercase().replace(' ', "-")),
            name: name.to_string(),
            aliases: vec![],
            pronouns: String::new(),
            email: vec![format!("{}@example.com", name.to_lowercase().replace(' ', "."))],
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
            relationship: None,
            tags: vec![],
            status,
            follow_up_cadence: String::new(),
            last_contacted: None,
            next_follow_up: None,
            priority: None,
            source: source.to_string(),
            source_id: source_id.to_string(),
            etag: etag.to_string(),
        }
    }

    fn make_contact_file(contact: Contact) -> ContactFile {
        ContactFile {
            path: PathBuf::from(format!("contacts/{}.md", contact.slug())),
            raw_frontmatter: String::new(),
            body: String::new(),
            contact,
        }
    }

    // === compute_push_changeset tests ===

    #[test]
    fn test_new_contact_no_source_goes_to_creates() {
        let tmp = tempfile::tempdir().unwrap();
        let cf = make_contact_file(make_contact("New Person", "", "", "", Some(Status::Active)));
        let server_entries: Vec<VCardEntry> = vec![];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.creates.len(), 1);
        assert_eq!(changeset.creates[0].contact.name, "New Person");
        assert_eq!(changeset.updates.len(), 0);
        assert_eq!(changeset.deletes.len(), 0);
        assert_eq!(changeset.conflicts.len(), 0);
    }

    #[test]
    fn test_icloud_contact_with_matching_cache_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = make_contact("Jane Smith", "icloud", "uid-123", "etag-abc", Some(Status::Active));

        // Write a cached vCard using the merge path (which is what compute_push_changeset uses)
        // First write an initial cache, then merge to get the canonical form
        let initial_vcard = vcard_write::contact_to_vcard(&contact).unwrap();
        let merged_vcard = vcard_write::merge_contact_to_vcard(&contact, &initial_vcard).unwrap();
        vcard_write::write_cached_vcard(tmp.path(), "uid-123", &merged_vcard).unwrap();

        let cf = make_contact_file(contact);
        let server_entries = vec![VCardEntry {
            href: "/path/uid-123.vcf".to_string(),
            etag: "etag-abc".to_string(),
        }];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.creates.len(), 0);
        assert_eq!(changeset.updates.len(), 0);
        assert_eq!(changeset.deletes.len(), 0);
        assert_eq!(changeset.conflicts.len(), 0);
    }

    #[test]
    fn test_icloud_contact_with_different_cache_goes_to_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = make_contact("Jane Smith", "icloud", "uid-123", "etag-abc", Some(Status::Active));

        // Write a cached vCard that does NOT match current serialization
        let old_vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Old Name\r\nN:Name;Old;;;\r\nUID:uid-123\r\nEND:VCARD\r\n";
        vcard_write::write_cached_vcard(tmp.path(), "uid-123", old_vcard).unwrap();

        let cf = make_contact_file(contact);
        let server_entries = vec![VCardEntry {
            href: "/path/uid-123.vcf".to_string(),
            etag: "etag-abc".to_string(),
        }];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.updates.len(), 1);
        assert_eq!(changeset.updates[0].0.contact.name, "Jane Smith");
        assert_eq!(changeset.creates.len(), 0);
        assert_eq!(changeset.deletes.len(), 0);
    }

    #[test]
    fn test_icloud_contact_with_etag_mismatch_goes_to_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = make_contact("Jane Smith", "icloud", "uid-123", "etag-local", Some(Status::Active));
        let cf = make_contact_file(contact);

        let server_entries = vec![VCardEntry {
            href: "/path/uid-123.vcf".to_string(),
            etag: "etag-server-different".to_string(),
        }];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.conflicts.len(), 1);
        assert_eq!(changeset.conflicts[0].0.contact.name, "Jane Smith");
        assert_eq!(changeset.conflicts[0].1, "etag-local");
        assert_eq!(changeset.conflicts[0].2, "etag-server-different");
        assert_eq!(changeset.creates.len(), 0);
        assert_eq!(changeset.updates.len(), 0);
    }

    #[test]
    fn test_archived_icloud_contact_goes_to_deletes() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = make_contact("Old Person", "icloud", "uid-456", "etag-old", Some(Status::Archived));
        let cf = make_contact_file(contact);

        let server_entries = vec![VCardEntry {
            href: "/path/uid-456.vcf".to_string(),
            etag: "etag-old".to_string(),
        }];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.deletes.len(), 1);
        assert_eq!(changeset.deletes[0].0, "uid-456");
        assert_eq!(changeset.deletes[0].1, "etag-old");
        assert_eq!(changeset.deletes[0].2, "Old Person");
    }

    #[test]
    fn test_icloud_contact_with_no_source_id_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = make_contact("Incomplete", "icloud", "", "", Some(Status::Active));
        let cf = make_contact_file(contact);

        let server_entries: Vec<VCardEntry> = vec![];

        let changeset = compute_push_changeset(tmp.path(), vec![cf], &server_entries).unwrap();

        assert_eq!(changeset.creates.len(), 0);
        assert_eq!(changeset.updates.len(), 0);
        assert_eq!(changeset.deletes.len(), 0);
        assert_eq!(changeset.conflicts.len(), 0);
    }

    #[test]
    fn test_push_result_tracks_counts() {
        let result = PushResult {
            created: 3,
            updated: 2,
            deleted: 1,
            conflicted: 1,
            failed: 0,
            details: vec![
                PushDetail { name: "A".to_string(), action: "created".to_string(), error: None },
                PushDetail { name: "B".to_string(), action: "updated".to_string(), error: None },
                PushDetail { name: "C".to_string(), action: "deleted".to_string(), error: None },
                PushDetail { name: "D".to_string(), action: "conflict".to_string(), error: Some("etag mismatch".to_string()) },
            ],
        };
        assert_eq!(result.created, 3);
        assert_eq!(result.updated, 2);
        assert_eq!(result.deleted, 1);
        assert_eq!(result.conflicted, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.details.len(), 4);
    }

    #[test]
    fn test_extract_uid_from_href() {
        assert_eq!(extract_uid_from_href("/123/carddavhome/card/ABC-DEF.vcf"), "ABC-DEF");
        assert_eq!(extract_uid_from_href("contact.vcf"), "contact");
        assert_eq!(extract_uid_from_href("/path/to/file"), "file");
    }

    #[test]
    fn test_mixed_contacts_categorized_correctly() {
        let tmp = tempfile::tempdir().unwrap();

        // New contact (no source)
        let new_contact = make_contact_file(make_contact("New Guy", "", "", "", Some(Status::Active)));

        // Unchanged icloud contact (matching cache - use merge path for canonical form)
        let unchanged = make_contact("Unchanged", "icloud", "uid-unch", "etag-1", Some(Status::Active));
        let initial_vcard = vcard_write::contact_to_vcard(&unchanged).unwrap();
        let merged_vcard = vcard_write::merge_contact_to_vcard(&unchanged, &initial_vcard).unwrap();
        vcard_write::write_cached_vcard(tmp.path(), "uid-unch", &merged_vcard).unwrap();
        let unchanged_cf = make_contact_file(unchanged);

        // Modified icloud contact (different cache)
        let modified = make_contact("Modified", "icloud", "uid-mod", "etag-2", Some(Status::Active));
        vcard_write::write_cached_vcard(tmp.path(), "uid-mod", "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Old\r\nN:Old;;;\r\nUID:uid-mod\r\nEND:VCARD\r\n").unwrap();
        let modified_cf = make_contact_file(modified);

        // Archived contact
        let archived_cf = make_contact_file(make_contact("Archived", "icloud", "uid-arch", "etag-3", Some(Status::Archived)));

        let server_entries = vec![
            VCardEntry { href: "/path/uid-unch.vcf".to_string(), etag: "etag-1".to_string() },
            VCardEntry { href: "/path/uid-mod.vcf".to_string(), etag: "etag-2".to_string() },
            VCardEntry { href: "/path/uid-arch.vcf".to_string(), etag: "etag-3".to_string() },
        ];

        let changeset = compute_push_changeset(
            tmp.path(),
            vec![new_contact, unchanged_cf, modified_cf, archived_cf],
            &server_entries,
        ).unwrap();

        assert_eq!(changeset.creates.len(), 1, "Should have 1 create");
        assert_eq!(changeset.updates.len(), 1, "Should have 1 update");
        assert_eq!(changeset.deletes.len(), 1, "Should have 1 delete");
        assert_eq!(changeset.conflicts.len(), 0, "Should have 0 conflicts");
    }
}
