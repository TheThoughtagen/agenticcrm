use crate::models::Contact;
use anyhow::Result;
use calcard::vcard::{VCard, VCardEntry, VCardProperty, VCardValue};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Properties the CRM maps -- these get removed during merge and re-added from Contact data.
const CRM_MAPPED_PROPERTIES: &[VCardProperty] = &[
    VCardProperty::Fn,
    VCardProperty::N,
    VCardProperty::Email,
    VCardProperty::Tel,
    VCardProperty::Org,
    VCardProperty::Title,
    VCardProperty::Url,
    VCardProperty::Bday,
    VCardProperty::Note,
];

/// Build a fresh vCard 3.0 string from a Contact.
pub fn contact_to_vcard(contact: &Contact) -> Result<String> {
    let mut entries = Vec::new();

    // VERSION:3.0 (required first)
    entries.push(
        VCardEntry::new(VCardProperty::Version)
            .with_value(VCardValue::Text("3.0".to_string())),
    );

    // FN (required in vCard 3.0)
    entries.push(
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text(contact.name.clone())),
    );

    // N (structured: Family;Given;;;)
    // Each component is a separate Text value, separated by ; in output
    let (given, family) = split_name(&contact.name);
    entries.push(
        VCardEntry::new(VCardProperty::N)
            .with_values(vec![
                VCardValue::Text(family.to_string()),
                VCardValue::Text(given.to_string()),
                VCardValue::Text(String::new()),
                VCardValue::Text(String::new()),
                VCardValue::Text(String::new()),
            ]),
    );

    // EMAIL entries (one per address)
    for email in &contact.email {
        entries.push(
            VCardEntry::new(VCardProperty::Email)
                .with_value(VCardValue::Text(email.clone())),
        );
    }

    // TEL entries (one per number)
    for phone in &contact.phone {
        entries.push(
            VCardEntry::new(VCardProperty::Tel)
                .with_value(VCardValue::Text(phone.clone())),
        );
    }

    // ORG (only if non-empty)
    if !contact.company.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Org)
                .with_value(VCardValue::Text(contact.company.clone())),
        );
    }

    // TITLE (only if non-empty)
    if !contact.role.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Title)
                .with_value(VCardValue::Text(contact.role.clone())),
        );
    }

    // URL (only if non-empty)
    if !contact.website.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Url)
                .with_value(VCardValue::Text(contact.website.clone())),
        );
    }

    // BDAY (only if set)
    if let Some(bday) = contact.birthday {
        entries.push(
            VCardEntry::new(VCardProperty::Bday)
                .with_value(VCardValue::Text(bday.format("%Y-%m-%d").to_string())),
        );
    }

    // UID (source_id if set, else contact.id)
    let uid = if !contact.source_id.is_empty() {
        &contact.source_id
    } else {
        &contact.id
    };
    entries.push(
        VCardEntry::new(VCardProperty::Uid)
            .with_value(VCardValue::Text(uid.clone())),
    );

    let vcard = VCard { entries };
    Ok(ensure_crlf(&vcard.to_string()))
}

/// Parse a cached vCard, replace CRM-mapped properties with current Contact data,
/// preserve all other properties (X-ABUID, PHOTO, etc.), and serialize.
pub fn merge_contact_to_vcard(contact: &Contact, cached_vcard_text: &str) -> Result<String> {
    let vcard = match VCard::parse(cached_vcard_text) {
        Ok(vcard) => vcard,
        Err(_) => {
            // Fall back to building from scratch if cached vCard is unparseable
            return contact_to_vcard(contact);
        }
    };

    // Retain entries NOT in CRM_MAPPED_PROPERTIES (preserves VERSION, UID, X-*, PHOTO, etc.)
    let mut retained: Vec<VCardEntry> = vcard
        .entries
        .into_iter()
        .filter(|entry| !CRM_MAPPED_PROPERTIES.contains(&entry.name))
        .collect();

    // Re-add CRM-mapped properties from current Contact data
    add_crm_entries(&mut retained, contact);

    let merged = VCard { entries: retained };
    Ok(ensure_crlf(&merged.to_string()))
}

/// Add CRM-mapped property entries to an existing entries list.
/// Does NOT add VERSION or UID (those should already be retained from the cached vCard).
fn add_crm_entries(entries: &mut Vec<VCardEntry>, contact: &Contact) {
    // FN
    entries.push(
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text(contact.name.clone())),
    );

    // N
    let (given, family) = split_name(&contact.name);
    entries.push(
        VCardEntry::new(VCardProperty::N)
            .with_value(VCardValue::Component(vec![
                family.to_string(),
                given.to_string(),
                String::new(),
                String::new(),
                String::new(),
            ])),
    );

    // EMAIL
    for email in &contact.email {
        entries.push(
            VCardEntry::new(VCardProperty::Email)
                .with_value(VCardValue::Text(email.clone())),
        );
    }

    // TEL
    for phone in &contact.phone {
        entries.push(
            VCardEntry::new(VCardProperty::Tel)
                .with_value(VCardValue::Text(phone.clone())),
        );
    }

    // ORG
    if !contact.company.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Org)
                .with_value(VCardValue::Text(contact.company.clone())),
        );
    }

    // TITLE
    if !contact.role.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Title)
                .with_value(VCardValue::Text(contact.role.clone())),
        );
    }

    // URL
    if !contact.website.is_empty() {
        entries.push(
            VCardEntry::new(VCardProperty::Url)
                .with_value(VCardValue::Text(contact.website.clone())),
        );
    }

    // BDAY
    if let Some(bday) = contact.birthday {
        entries.push(
            VCardEntry::new(VCardProperty::Bday)
                .with_value(VCardValue::Text(bday.format("%Y-%m-%d").to_string())),
        );
    }
}

/// A snapshot of the CRM-relevant fields of a Contact for semantic comparison.
/// Used to detect whether a contact has actually changed since last pull/push,
/// avoiding false positives from vCard serialization differences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactSnapshot {
    pub name: String,
    pub email: Vec<String>,
    pub phone: Vec<String>,
    pub company: String,
    pub role: String,
    pub website: String,
    pub birthday: Option<NaiveDate>,
}

impl From<&Contact> for ContactSnapshot {
    fn from(contact: &Contact) -> Self {
        ContactSnapshot {
            name: contact.name.clone(),
            email: contact.email.clone(),
            phone: contact.phone.clone(),
            company: contact.company.clone(),
            role: contact.role.clone(),
            website: contact.website.clone(),
            birthday: contact.birthday,
        }
    }
}

/// Returns the contact snapshot cache directory path.
pub fn contact_snapshot_dir(crm_root: &Path) -> PathBuf {
    crm_root.join(".sync").join("contact-snapshots")
}

/// Cache a Contact's CRM-relevant fields as a JSON snapshot for later comparison.
pub fn cache_contact_snapshot(crm_root: &Path, source_id: &str, contact: &Contact) -> Result<()> {
    let dir = contact_snapshot_dir(crm_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", source_id));
    let snapshot = ContactSnapshot::from(contact);
    let json = serde_json::to_string_pretty(&snapshot)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Read a cached Contact snapshot. Returns None if file does not exist.
pub fn read_contact_snapshot(crm_root: &Path, source_id: &str) -> Option<ContactSnapshot> {
    let path = contact_snapshot_dir(crm_root).join(format!("{}.json", source_id));
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Check whether a Contact's CRM-relevant fields have changed since the last snapshot.
/// Returns true if fields have changed or no snapshot exists (conservative: assume changed).
pub fn contact_fields_changed(crm_root: &Path, source_id: &str, contact: &Contact) -> bool {
    match read_contact_snapshot(crm_root, source_id) {
        Some(cached) => {
            let current = ContactSnapshot::from(contact);
            current != cached
        }
        None => true, // No snapshot = assume changed (first push)
    }
}

/// Split a name into (given, family) parts.
/// "Jane Smith" -> ("Jane", "Smith")
/// "Bob" -> ("Bob", "")
fn split_name(name: &str) -> (&str, &str) {
    match name.splitn(2, ' ').collect::<Vec<_>>().as_slice() {
        [first, last] => (first, last),
        [single] => (single, ""),
        _ => ("", ""),
    }
}

/// Returns the vCard cache directory path.
pub fn cache_dir(crm_root: &Path) -> PathBuf {
    crm_root.join(".sync").join("vcards")
}

/// Read a cached vCard file. Returns None if file does not exist.
pub fn read_cached_vcard(crm_root: &Path, source_id: &str) -> Option<String> {
    let path = cache_dir(crm_root).join(format!("{}.vcf", source_id));
    std::fs::read_to_string(path).ok()
}

/// Write a vCard to the cache directory, creating it if needed.
pub fn write_cached_vcard(crm_root: &Path, source_id: &str, vcard_text: &str) -> Result<()> {
    let dir = cache_dir(crm_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.vcf", source_id));
    std::fs::write(path, vcard_text)?;
    Ok(())
}

/// Delete a cached vCard file. No error if file is missing.
pub fn delete_cached_vcard(crm_root: &Path, source_id: &str) -> Result<()> {
    let path = cache_dir(crm_root).join(format!("{}.vcf", source_id));
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Ensure output uses CRLF line endings without double-converting.
fn ensure_crlf(text: &str) -> String {
    // Replace bare \n (not preceded by \r) with \r\n
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' && (i == 0 || bytes[i - 1] != b'\r') {
            result.push('\r');
        }
        result.push(b as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Contact, Status};
    use chrono::NaiveDate;

    fn test_contact() -> Contact {
        Contact {
            id: "test-id-123".to_string(),
            name: "Jane Smith".to_string(),
            aliases: vec![],
            pronouns: String::new(),
            email: vec!["jane@example.com".to_string(), "jane@work.com".to_string()],
            phone: vec!["+1-555-0100".to_string(), "+1-555-0200".to_string()],
            address: vec![],
            company: "Acme Corp".to_string(),
            role: "Engineer".to_string(),
            industry: String::new(),
            linkedin: String::new(),
            twitter: String::new(),
            facebook: String::new(),
            instagram: String::new(),
            github: String::new(),
            website: "https://jane.example.com".to_string(),
            birthday: Some(NaiveDate::from_ymd_opt(1990, 5, 15).unwrap()),
            interests: vec![],
            family: vec![],
            how_we_met: String::new(),
            met_date: None,
            introduced_by: String::new(),
            relationship: None,
            tags: vec![],
            status: Some(Status::Active),
            follow_up_cadence: String::new(),
            last_contacted: None,
            next_follow_up: None,
            priority: None,
            source: "icloud".to_string(),
            source_id: "uid-abc-123".to_string(),
            etag: "etag-xyz".to_string(),
        }
    }

    fn minimal_contact() -> Contact {
        Contact {
            id: "min-id".to_string(),
            name: "Bob".to_string(),
            aliases: vec![],
            pronouns: String::new(),
            email: vec!["bob@example.com".to_string()],
            phone: vec!["+1-555-9999".to_string()],
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
            status: None,
            follow_up_cadence: String::new(),
            last_contacted: None,
            next_follow_up: None,
            priority: None,
            source: String::new(),
            source_id: String::new(),
            etag: String::new(),
        }
    }

    // === contact_to_vcard tests ===

    #[test]
    fn test_contact_to_vcard_valid_vcard30() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(vcard_text.contains("BEGIN:VCARD"));
        assert!(vcard_text.contains("VERSION:3.0"));
        assert!(vcard_text.contains("END:VCARD"));
    }

    #[test]
    fn test_contact_to_vcard_has_fn_and_uid() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(vcard_text.contains("FN:Jane Smith"));
        assert!(vcard_text.contains("UID:uid-abc-123"));
    }

    #[test]
    fn test_contact_to_vcard_has_n_structured() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        // N should be Family;Given;;;
        assert!(vcard_text.contains("N:Smith;Jane;;;"));
    }

    #[test]
    fn test_contact_to_vcard_multiple_emails() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        // Each email should have its own EMAIL entry
        let email_count = vcard_text.matches("EMAIL:").count();
        assert_eq!(email_count, 2);
        assert!(vcard_text.contains("EMAIL:jane@example.com"));
        assert!(vcard_text.contains("EMAIL:jane@work.com"));
    }

    #[test]
    fn test_contact_to_vcard_multiple_phones() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        let tel_count = vcard_text.matches("TEL:").count();
        assert_eq!(tel_count, 2);
        assert!(vcard_text.contains("TEL:+1-555-0100"));
        assert!(vcard_text.contains("TEL:+1-555-0200"));
    }

    #[test]
    fn test_contact_to_vcard_org_title_url() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(vcard_text.contains("ORG:Acme Corp"));
        assert!(vcard_text.contains("TITLE:Engineer"));
        assert!(vcard_text.contains("URL:https://jane.example.com"));
    }

    #[test]
    fn test_contact_to_vcard_birthday() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(vcard_text.contains("BDAY:1990-05-15"));
    }

    #[test]
    fn test_contact_to_vcard_empty_fields_omitted() {
        let contact = minimal_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(!vcard_text.contains("ORG:"));
        assert!(!vcard_text.contains("TITLE:"));
        assert!(!vcard_text.contains("URL:"));
        assert!(!vcard_text.contains("BDAY:"));
    }

    #[test]
    fn test_contact_to_vcard_uses_contact_id_when_no_source_id() {
        let contact = minimal_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        assert!(vcard_text.contains("UID:min-id"));
    }

    #[test]
    fn test_contact_to_vcard_crlf_line_endings() {
        let contact = test_contact();
        let vcard_text = contact_to_vcard(&contact).unwrap();
        // Every \n should be preceded by \r
        for (i, b) in vcard_text.bytes().enumerate() {
            if b == b'\n' {
                assert!(i > 0 && vcard_text.as_bytes()[i - 1] == b'\r',
                    "Found bare LF at byte position {}", i);
            }
        }
    }

    // === merge_contact_to_vcard tests ===

    #[test]
    fn test_merge_preserves_non_crm_properties() {
        let contact = test_contact();
        let cached = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      FN:Old Name\r\n\
                      N:Old;Name;;;\r\n\
                      EMAIL:old@example.com\r\n\
                      UID:uid-abc-123\r\n\
                      X-ABUID:some-apple-uid\r\n\
                      X-ABLABEL:custom-label\r\n\
                      PHOTO;ENCODING=BASE64:iVBORSomeData\r\n\
                      END:VCARD";
        let result = merge_contact_to_vcard(&contact, cached).unwrap();
        // Non-CRM properties should survive
        assert!(result.contains("X-ABUID:some-apple-uid"));
        assert!(result.contains("X-ABLABEL:custom-label"));
        assert!(result.contains("PHOTO"));
        // CRM properties should reflect current contact data
        assert!(result.contains("FN:Jane Smith"));
        assert!(!result.contains("FN:Old Name"));
    }

    #[test]
    fn test_merge_preserves_version_and_uid() {
        let contact = test_contact();
        let cached = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      FN:Old Name\r\n\
                      UID:uid-abc-123\r\n\
                      END:VCARD";
        let result = merge_contact_to_vcard(&contact, cached).unwrap();
        assert!(result.contains("VERSION:3.0"));
        assert!(result.contains("UID:uid-abc-123"));
    }

    #[test]
    fn test_merge_replaces_crm_properties() {
        let contact = test_contact();
        let cached = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      FN:Old Name\r\n\
                      N:Old;Name;;;\r\n\
                      EMAIL:old@example.com\r\n\
                      TEL:+1-000-0000\r\n\
                      ORG:Old Corp\r\n\
                      TITLE:Old Title\r\n\
                      URL:https://old.example.com\r\n\
                      BDAY:1980-01-01\r\n\
                      UID:uid-abc-123\r\n\
                      END:VCARD";
        let result = merge_contact_to_vcard(&contact, cached).unwrap();
        assert!(result.contains("FN:Jane Smith"));
        assert!(result.contains("EMAIL:jane@example.com"));
        assert!(result.contains("EMAIL:jane@work.com"));
        assert!(result.contains("TEL:+1-555-0100"));
        assert!(result.contains("ORG:Acme Corp"));
        assert!(result.contains("TITLE:Engineer"));
        assert!(result.contains("BDAY:1990-05-15"));
        // Old values should be gone
        assert!(!result.contains("Old Name"));
        assert!(!result.contains("old@example.com"));
        assert!(!result.contains("Old Corp"));
    }

    #[test]
    fn test_merge_fallback_on_bad_parse() {
        let contact = test_contact();
        let bad_cached = "this is not a vcard at all";
        // Should fall back to contact_to_vcard
        let result = merge_contact_to_vcard(&contact, bad_cached).unwrap();
        assert!(result.contains("FN:Jane Smith"));
        assert!(result.contains("VERSION:3.0"));
    }

    // === Cache function tests ===

    #[test]
    fn test_read_cached_vcard_returns_none_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let result = read_cached_vcard(tmp.path(), "nonexistent-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_write_and_read_cached_vcard() {
        let tmp = tempfile::tempdir().unwrap();
        let vcard_text = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Test\r\nEND:VCARD\r\n";
        write_cached_vcard(tmp.path(), "test-source-id", vcard_text).unwrap();
        let read_back = read_cached_vcard(tmp.path(), "test-source-id");
        assert_eq!(read_back, Some(vcard_text.to_string()));
    }

    #[test]
    fn test_write_cached_vcard_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let vcard_text = "BEGIN:VCARD\r\nEND:VCARD\r\n";
        write_cached_vcard(tmp.path(), "new-id", vcard_text).unwrap();
        assert!(cache_dir(tmp.path()).exists());
    }

    #[test]
    fn test_delete_cached_vcard_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let vcard_text = "BEGIN:VCARD\r\nEND:VCARD\r\n";
        write_cached_vcard(tmp.path(), "del-id", vcard_text).unwrap();
        assert!(read_cached_vcard(tmp.path(), "del-id").is_some());
        delete_cached_vcard(tmp.path(), "del-id").unwrap();
        assert!(read_cached_vcard(tmp.path(), "del-id").is_none());
    }

    #[test]
    fn test_delete_cached_vcard_no_error_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // Should not error
        delete_cached_vcard(tmp.path(), "never-existed").unwrap();
    }

    // === ContactSnapshot and contact_fields_changed tests ===

    #[test]
    fn test_contact_snapshot_unchanged_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();
        cache_contact_snapshot(tmp.path(), "uid-abc-123", &contact).unwrap();
        assert!(!contact_fields_changed(tmp.path(), "uid-abc-123", &contact));
    }

    #[test]
    fn test_contact_fields_changed_name_differs() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();
        cache_contact_snapshot(tmp.path(), "uid-abc-123", &contact).unwrap();

        let mut modified = contact;
        modified.name = "Jane Doe".to_string();
        assert!(contact_fields_changed(tmp.path(), "uid-abc-123", &modified));
    }

    #[test]
    fn test_contact_fields_changed_email_differs() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();
        cache_contact_snapshot(tmp.path(), "uid-abc-123", &contact).unwrap();

        let mut modified = contact;
        modified.email = vec!["new@example.com".to_string()];
        assert!(contact_fields_changed(tmp.path(), "uid-abc-123", &modified));
    }

    #[test]
    fn test_contact_fields_changed_phone_company_role_website_birthday() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();

        // Phone
        cache_contact_snapshot(tmp.path(), "uid-1", &contact).unwrap();
        let mut m = contact.clone();
        m.phone = vec!["+1-999-9999".to_string()];
        assert!(contact_fields_changed(tmp.path(), "uid-1", &m));

        // Company
        cache_contact_snapshot(tmp.path(), "uid-2", &contact).unwrap();
        let mut m = contact.clone();
        m.company = "New Corp".to_string();
        assert!(contact_fields_changed(tmp.path(), "uid-2", &m));

        // Role
        cache_contact_snapshot(tmp.path(), "uid-3", &contact).unwrap();
        let mut m = contact.clone();
        m.role = "Manager".to_string();
        assert!(contact_fields_changed(tmp.path(), "uid-3", &m));

        // Website
        cache_contact_snapshot(tmp.path(), "uid-4", &contact).unwrap();
        let mut m = contact.clone();
        m.website = "https://new.example.com".to_string();
        assert!(contact_fields_changed(tmp.path(), "uid-4", &m));

        // Birthday
        cache_contact_snapshot(tmp.path(), "uid-5", &contact).unwrap();
        let mut m = contact.clone();
        m.birthday = Some(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
        assert!(contact_fields_changed(tmp.path(), "uid-5", &m));
    }

    #[test]
    fn test_contact_fields_changed_no_snapshot_returns_true() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();
        // No snapshot cached -- should assume changed
        assert!(contact_fields_changed(tmp.path(), "uid-abc-123", &contact));
    }

    #[test]
    fn test_contact_snapshot_cache_and_read_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let contact = test_contact();
        cache_contact_snapshot(tmp.path(), "uid-abc-123", &contact).unwrap();
        let snapshot = read_contact_snapshot(tmp.path(), "uid-abc-123");
        assert!(snapshot.is_some());
        let snap = snapshot.unwrap();
        assert_eq!(snap.name, "Jane Smith");
        assert_eq!(snap.email, vec!["jane@example.com", "jane@work.com"]);
        assert_eq!(snap.company, "Acme Corp");
    }

    // === ensure_crlf tests ===

    #[test]
    fn test_ensure_crlf_converts_bare_lf() {
        assert_eq!(ensure_crlf("a\nb\nc"), "a\r\nb\r\nc");
    }

    #[test]
    fn test_ensure_crlf_preserves_existing_crlf() {
        assert_eq!(ensure_crlf("a\r\nb\r\nc"), "a\r\nb\r\nc");
    }

    #[test]
    fn test_ensure_crlf_mixed() {
        assert_eq!(ensure_crlf("a\r\nb\nc"), "a\r\nb\r\nc");
    }
}
