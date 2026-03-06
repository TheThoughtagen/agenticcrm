use std::fmt;

use crate::models::Contact;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

const VALID_CADENCES: &[&str] = &["weekly", "biweekly", "monthly", "quarterly", "yearly"];

/// Validate a contact, returning a list of validation errors.
/// Returns an empty vec for a valid contact.
pub fn validate_contact(contact: &Contact) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if contact.id.is_empty() {
        errors.push(ValidationError {
            field: "id".to_string(),
            message: "must not be empty".to_string(),
        });
    }

    if contact.name.is_empty() {
        errors.push(ValidationError {
            field: "name".to_string(),
            message: "must not be empty".to_string(),
        });
    }

    if !contact.follow_up_cadence.is_empty()
        && !VALID_CADENCES.contains(&contact.follow_up_cadence.as_str())
    {
        errors.push(ValidationError {
            field: "follow_up_cadence".to_string(),
            message: format!(
                "invalid value '{}', must be one of: {}",
                contact.follow_up_cadence,
                VALID_CADENCES.join(", ")
            ),
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Contact, Status};

    fn valid_contact() -> Contact {
        Contact {
            id: "test-id".to_string(),
            name: "Test Name".to_string(),
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
            source: String::new(),
            source_id: String::new(),
        }
    }

    #[test]
    fn test_validate_valid_contact() {
        let errors = validate_contact(&valid_contact());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_empty_name() {
        let mut c = valid_contact();
        c.name = String::new();
        let errors = validate_contact(&c);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn test_validate_empty_id() {
        let mut c = valid_contact();
        c.id = String::new();
        let errors = validate_contact(&c);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "id");
    }

    #[test]
    fn test_validate_invalid_cadence() {
        let mut c = valid_contact();
        c.follow_up_cadence = "daily".to_string();
        let errors = validate_contact(&c);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "follow_up_cadence");
    }

    #[test]
    fn test_validate_valid_cadence() {
        let mut c = valid_contact();
        c.follow_up_cadence = "monthly".to_string();
        let errors = validate_contact(&c);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError {
            field: "name".to_string(),
            message: "must not be empty".to_string(),
        };
        assert_eq!(format!("{err}"), "name: must not be empty");
    }
}
