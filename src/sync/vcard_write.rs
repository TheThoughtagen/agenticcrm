use crate::models::Contact;
use anyhow::Result;
use calcard::vcard::{VCard, VCardEntry, VCardProperty, VCardValue};
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
pub fn contact_to_vcard(_contact: &Contact) -> Result<String> {
    todo!()
}

/// Parse a cached vCard, replace CRM-mapped properties with current Contact data,
/// preserve all other properties (X-ABUID, PHOTO, etc.), and serialize.
pub fn merge_contact_to_vcard(_contact: &Contact, _cached_vcard_text: &str) -> Result<String> {
    todo!()
}

/// Returns the vCard cache directory path.
pub fn cache_dir(crm_root: &Path) -> PathBuf {
    crm_root.join(".sync").join("vcards")
}

/// Read a cached vCard file. Returns None if file does not exist.
pub fn read_cached_vcard(_crm_root: &Path, _source_id: &str) -> Option<String> {
    todo!()
}

/// Write a vCard to the cache directory, creating it if needed.
pub fn write_cached_vcard(_crm_root: &Path, _source_id: &str, _vcard_text: &str) -> Result<()> {
    todo!()
}

/// Delete a cached vCard file. No error if file is missing.
pub fn delete_cached_vcard(_crm_root: &Path, _source_id: &str) -> Result<()> {
    todo!()
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
