use crate::models::ContactFile;

/// Find an existing contact by its source_id (CardDAV UID).
/// Returns the first matching ContactFile, if any.
pub fn find_existing_by_source_id<'a>(
    contacts: &'a [ContactFile],
    source_id: &str,
) -> Option<&'a ContactFile> {
    contacts.iter().find(|cf| cf.contact.source_id == source_id)
}

/// Determine whether an existing contact should be updated.
/// Returns true if the stored etag differs from the new etag (server has newer version).
pub fn should_update(existing: &ContactFile, new_etag: &str) -> bool {
    existing.contact.etag != new_etag
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Contact, Status};
    use std::path::PathBuf;

    fn make_contact_file(source_id: &str, etag: &str) -> ContactFile {
        ContactFile {
            contact: Contact {
                id: "test-id".to_string(),
                name: "Test".to_string(),
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
                relationship: None,
                tags: vec![],
                status: Some(Status::Active),
                follow_up_cadence: String::new(),
                last_contacted: None,
                next_follow_up: None,
                priority: None,
                source: "icloud".to_string(),
                source_id: source_id.to_string(),
                etag: etag.to_string(),
            },
            body: String::new(),
            path: PathBuf::from("contacts/test.md"),
            raw_frontmatter: String::new(),
        }
    }

    #[test]
    fn test_find_existing_by_source_id_match() {
        let contacts = vec![
            make_contact_file("uid-aaa", "etag-1"),
            make_contact_file("uid-bbb", "etag-2"),
        ];
        let result = find_existing_by_source_id(&contacts, "uid-bbb");
        assert!(result.is_some());
        assert_eq!(result.unwrap().contact.source_id, "uid-bbb");
    }

    #[test]
    fn test_find_existing_by_source_id_no_match() {
        let contacts = vec![
            make_contact_file("uid-aaa", "etag-1"),
        ];
        let result = find_existing_by_source_id(&contacts, "uid-zzz");
        assert!(result.is_none());
    }

    #[test]
    fn test_should_update_etag_differs() {
        let cf = make_contact_file("uid-1", "old-etag");
        assert!(should_update(&cf, "new-etag"));
    }

    #[test]
    fn test_should_update_etag_matches() {
        let cf = make_contact_file("uid-1", "same-etag");
        assert!(!should_update(&cf, "same-etag"));
    }
}
