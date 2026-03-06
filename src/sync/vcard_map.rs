use crate::models::{Contact, Status};
use anyhow::Result;
use calcard::vcard::{VCard, VCardProperty, VCardValue};
use chrono::NaiveDate;

/// Result of mapping a vCard to our Contact model
pub struct MappedContact {
    pub contact: Contact,
    pub notes: String,
}

/// Map a raw vCard text string to a Contact, using the given UID and ETag.
///
/// Name fallback chain: FN -> N (reconstructed as "Given Family") -> ORG -> first EMAIL -> "Unknown Contact"
pub fn map_vcard_to_contact(vcard_text: &str, uid: &str, etag: &str) -> Result<MappedContact> {
    let vcard = VCard::parse(vcard_text).map_err(|e| anyhow::anyhow!("vCard parse error: {:?}", e))?;

    // Name fallback chain
    let name = extract_fn(&vcard)
        .or_else(|| extract_n_reconstructed(&vcard))
        .or_else(|| extract_org(&vcard))
        .or_else(|| extract_first_email(&vcard))
        .unwrap_or_else(|| "Unknown Contact".to_string());

    let contact = Contact {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        aliases: vec![],
        pronouns: String::new(),
        email: extract_all_values(&vcard, &VCardProperty::Email),
        phone: extract_all_values(&vcard, &VCardProperty::Tel),
        address: vec![],
        company: extract_org(&vcard).unwrap_or_default(),
        role: extract_first_text(&vcard, &VCardProperty::Title).unwrap_or_default(),
        industry: String::new(),
        linkedin: String::new(),
        twitter: String::new(),
        facebook: String::new(),
        instagram: String::new(),
        github: String::new(),
        website: extract_first_text(&vcard, &VCardProperty::Url).unwrap_or_default(),
        birthday: extract_birthday(&vcard),
        interests: vec![],
        family: vec![],
        how_we_met: String::new(),
        met_date: None,
        introduced_by: String::new(),
        relationship: None,
        tags: vec!["icloud-import".to_string()],
        status: Some(Status::Active),
        follow_up_cadence: String::new(),
        last_contacted: None,
        next_follow_up: None,
        priority: None,
        source: "icloud".to_string(),
        source_id: uid.to_string(),
        etag: etag.to_string(),
    };

    let notes = extract_first_text(&vcard, &VCardProperty::Note).unwrap_or_default();

    Ok(MappedContact { contact, notes })
}

/// Extract the FN (formatted name) property
fn extract_fn(vcard: &VCard) -> Option<String> {
    extract_first_text(vcard, &VCardProperty::Fn)
        .filter(|s| !s.is_empty())
}

/// Extract and reconstruct the N (structured name) as "Given Family"
fn extract_n_reconstructed(vcard: &VCard) -> Option<String> {
    let entry = vcard.property(&VCardProperty::N)?;
    // N is structured: Family;Given;Middle;Prefix;Suffix
    // Values come as Component or multiple Text values
    let parts: Vec<String> = entry.values.iter().filter_map(|v| {
        match v {
            VCardValue::Text(s) => Some(s.clone()),
            VCardValue::Component(parts) => Some(parts.join(" ")),
            _ => None,
        }
    }).collect();

    if parts.is_empty() {
        return None;
    }

    // N structure: [family, given, middle, prefix, suffix]
    let family = parts.first().map(|s| s.as_str()).unwrap_or("");
    let given = parts.get(1).map(|s| s.as_str()).unwrap_or("");

    let name = format!("{} {}", given, family).trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}

/// Extract the first component of ORG
fn extract_org(vcard: &VCard) -> Option<String> {
    let entry = vcard.property(&VCardProperty::Org)?;
    entry.values.first().and_then(|v| {
        match v {
            VCardValue::Text(s) if !s.is_empty() => Some(s.clone()),
            VCardValue::Component(parts) => parts.first().filter(|s| !s.is_empty()).cloned(),
            _ => None,
        }
    })
}

/// Extract the first email value
fn extract_first_email(vcard: &VCard) -> Option<String> {
    extract_first_text(vcard, &VCardProperty::Email)
        .filter(|s| !s.is_empty())
}

/// Extract the first text value for a given property
fn extract_first_text(vcard: &VCard, prop: &VCardProperty) -> Option<String> {
    vcard.property(prop)
        .and_then(|entry| entry.values.first())
        .and_then(|v| v.as_text())
        .map(|s| s.to_string())
}

/// Extract all text values for a property (handles multiple entries like EMAIL, TEL)
fn extract_all_values(vcard: &VCard, prop: &VCardProperty) -> Vec<String> {
    vcard.properties(prop)
        .filter_map(|entry| {
            entry.values.first().and_then(|v| v.as_text()).map(|s| s.to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract birthday from BDAY property
fn extract_birthday(vcard: &VCard) -> Option<NaiveDate> {
    let entry = vcard.property(&VCardProperty::Bday)?;
    let value = entry.values.first()?;

    match value {
        VCardValue::PartialDateTime(pdt) => {
            let year = pdt.year? as i32;
            let month = pdt.month? as u32;
            let day = pdt.day? as u32;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        VCardValue::Text(s) => {
            // Try YYYY-MM-DD
            NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                .or_else(|| NaiveDate::parse_from_str(s, "%Y%m%d").ok())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_vcard() -> &'static str {
        "BEGIN:VCARD\r\n\
         VERSION:3.0\r\n\
         FN:Jane Smith\r\n\
         N:Smith;Jane;;;\r\n\
         EMAIL:jane@example.com\r\n\
         EMAIL:jane.smith@work.com\r\n\
         TEL:+1-555-0100\r\n\
         TEL:+1-555-0200\r\n\
         ORG:Acme Corp\r\n\
         TITLE:Engineer\r\n\
         URL:https://jane.example.com\r\n\
         BDAY:1990-05-15\r\n\
         NOTE:Met at conference\r\n\
         UID:abc-123\r\n\
         END:VCARD"
    }

    #[test]
    fn test_map_full_vcard() {
        let result = map_vcard_to_contact(full_vcard(), "uid-123", "etag-abc").unwrap();
        assert_eq!(result.contact.name, "Jane Smith");
        assert_eq!(result.contact.company, "Acme Corp");
        assert_eq!(result.contact.role, "Engineer");
        assert_eq!(result.contact.website, "https://jane.example.com");
        assert_eq!(result.notes, "Met at conference");
    }

    #[test]
    fn test_map_vcard_fn_missing_falls_back_to_n() {
        let vcard = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      N:Doe;John;;;\r\n\
                      EMAIL:john@example.com\r\n\
                      END:VCARD";
        let result = map_vcard_to_contact(vcard, "uid-1", "etag-1").unwrap();
        assert_eq!(result.contact.name, "John Doe");
    }

    #[test]
    fn test_map_vcard_no_name_falls_back_to_org() {
        let vcard = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      ORG:Widgets Inc\r\n\
                      EMAIL:info@widgets.com\r\n\
                      END:VCARD";
        let result = map_vcard_to_contact(vcard, "uid-2", "etag-2").unwrap();
        assert_eq!(result.contact.name, "Widgets Inc");
    }

    #[test]
    fn test_map_vcard_no_name_falls_back_to_email() {
        let vcard = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      EMAIL:mystery@example.com\r\n\
                      END:VCARD";
        let result = map_vcard_to_contact(vcard, "uid-3", "etag-3").unwrap();
        assert_eq!(result.contact.name, "mystery@example.com");
    }

    #[test]
    fn test_map_vcard_no_name_falls_back_to_unknown() {
        let vcard = "BEGIN:VCARD\r\n\
                      VERSION:3.0\r\n\
                      TEL:+1-555-0000\r\n\
                      END:VCARD";
        let result = map_vcard_to_contact(vcard, "uid-4", "etag-4").unwrap();
        assert_eq!(result.contact.name, "Unknown Contact");
    }

    #[test]
    fn test_map_vcard_sets_source_fields() {
        let result = map_vcard_to_contact(full_vcard(), "my-uid", "my-etag").unwrap();
        assert_eq!(result.contact.source, "icloud");
        assert_eq!(result.contact.source_id, "my-uid");
        assert_eq!(result.contact.etag, "my-etag");
    }

    #[test]
    fn test_map_vcard_adds_icloud_import_tag() {
        let result = map_vcard_to_contact(full_vcard(), "uid", "etag").unwrap();
        assert!(result.contact.tags.contains(&"icloud-import".to_string()));
    }

    #[test]
    fn test_map_vcard_multiple_emails_and_phones() {
        let result = map_vcard_to_contact(full_vcard(), "uid", "etag").unwrap();
        assert_eq!(result.contact.email.len(), 2);
        assert_eq!(result.contact.email[0], "jane@example.com");
        assert_eq!(result.contact.email[1], "jane.smith@work.com");
        assert_eq!(result.contact.phone.len(), 2);
        assert_eq!(result.contact.phone[0], "+1-555-0100");
        assert_eq!(result.contact.phone[1], "+1-555-0200");
    }
}
