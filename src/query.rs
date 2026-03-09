use crate::models::contact::Contact;
use crate::ops::OpsError;

/// Comparison operator for a query predicate
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Exact match (case-insensitive); for arrays, "any element equals"
    Eq,
    /// Not equal (case-insensitive)
    NotEq,
    /// Substring match (case-insensitive); for arrays, "any element contains substring"
    Contains,
}

/// A single field predicate: field op value
#[derive(Debug, Clone)]
pub struct Predicate {
    pub field: String,
    pub op: Op,
    pub value: String,
}

/// A parsed query consisting of multiple predicates (implicit AND)
#[derive(Debug, Clone)]
pub struct Query {
    pub predicates: Vec<Predicate>,
}

/// Represents a field value extracted from a Contact
#[derive(Debug)]
pub enum FieldValue {
    Single(String),
    List(Vec<String>),
    None,
}

impl Query {
    /// Parse a query string into predicates.
    ///
    /// Supports operators: `!=`, `~`, `=` (checked in that order).
    /// Tokens are separated by whitespace or commas.
    ///
    /// Examples:
    /// - `"status=dormant"` -> one Eq predicate
    /// - `"status!=active"` -> one NotEq predicate
    /// - `"tags~friend"` -> one Contains predicate
    /// - `"status=dormant tags~friend"` -> two predicates (AND)
    /// - `"status=dormant,tags~friend"` -> two predicates (AND)
    pub fn parse(input: &str) -> Result<Self, OpsError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(OpsError::ValidationFailed(
                "Empty query. Use field=value, field!=value, or field~value".to_string(),
            ));
        }

        // Split on whitespace and commas
        let tokens: Vec<&str> = input
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .collect();

        let mut predicates = Vec::new();

        for token in tokens {
            // Check for != first (before =)
            if let Some(pos) = token.find("!=") {
                let field = token[..pos].to_string();
                let value = token[pos + 2..].to_string();
                if field.is_empty() || value.is_empty() {
                    return Err(OpsError::ValidationFailed(format!(
                        "Invalid predicate '{token}': field and value required"
                    )));
                }
                predicates.push(Predicate {
                    field,
                    op: Op::NotEq,
                    value,
                });
            } else if let Some(pos) = token.find('~') {
                let field = token[..pos].to_string();
                let value = token[pos + 1..].to_string();
                if field.is_empty() || value.is_empty() {
                    return Err(OpsError::ValidationFailed(format!(
                        "Invalid predicate '{token}': field and value required"
                    )));
                }
                predicates.push(Predicate {
                    field,
                    op: Op::Contains,
                    value,
                });
            } else if let Some(pos) = token.find('=') {
                let field = token[..pos].to_string();
                let value = token[pos + 1..].to_string();
                if field.is_empty() || value.is_empty() {
                    return Err(OpsError::ValidationFailed(format!(
                        "Invalid predicate '{token}': field and value required"
                    )));
                }
                predicates.push(Predicate {
                    field,
                    op: Op::Eq,
                    value,
                });
            } else {
                return Err(OpsError::ValidationFailed(format!(
                    "No operator found in '{token}'. Use field=value, field!=value, or field~value"
                )));
            }
        }

        Ok(Query { predicates })
    }

    /// Check if a contact matches all predicates in this query (implicit AND).
    pub fn matches(&self, contact: &Contact) -> bool {
        self.predicates.iter().all(|pred| {
            let field_val = get_field_value(contact, &pred.field);
            match (&pred.op, &field_val) {
                (Op::Eq, FieldValue::Single(s)) => {
                    s.to_lowercase() == pred.value.to_lowercase()
                }
                (Op::Eq, FieldValue::List(items)) => {
                    // For arrays, Eq means "any element equals value"
                    items.iter().any(|item| {
                        item.to_lowercase() == pred.value.to_lowercase()
                    })
                }
                (Op::NotEq, FieldValue::Single(s)) => {
                    s.to_lowercase() != pred.value.to_lowercase()
                }
                (Op::NotEq, FieldValue::List(items)) => {
                    // NotEq for arrays: no element equals value
                    !items.iter().any(|item| {
                        item.to_lowercase() == pred.value.to_lowercase()
                    })
                }
                (Op::Contains, FieldValue::Single(s)) => {
                    s.to_lowercase().contains(&pred.value.to_lowercase())
                }
                (Op::Contains, FieldValue::List(items)) => {
                    // Contains for arrays: any element contains substring
                    items.iter().any(|item| {
                        item.to_lowercase().contains(&pred.value.to_lowercase())
                    })
                }
                (_, FieldValue::None) => false,
            }
        })
    }
}

/// Extract a field value from a Contact by field name.
///
/// For enum fields (Status, Relationship, Priority), returns the serde-serialized
/// string representation (e.g., "lost-touch" for Status::LostTouch).
///
/// Unknown fields return FieldValue::None.
pub fn get_field_value(contact: &Contact, field: &str) -> FieldValue {
    match field {
        // Identity
        "id" => FieldValue::Single(contact.id.clone()),
        "name" => FieldValue::Single(contact.name.clone()),
        "aliases" => FieldValue::List(contact.aliases.clone()),
        "pronouns" => FieldValue::Single(contact.pronouns.clone()),

        // Contact info
        "email" => FieldValue::List(contact.email.clone()),
        "phone" => FieldValue::List(contact.phone.clone()),
        "address" => FieldValue::List(contact.address.clone()),

        // Professional
        "company" => FieldValue::Single(contact.company.clone()),
        "role" => FieldValue::Single(contact.role.clone()),
        "industry" => FieldValue::Single(contact.industry.clone()),
        "linkedin" => FieldValue::Single(contact.linkedin.clone()),

        // Social
        "twitter" => FieldValue::Single(contact.twitter.clone()),
        "facebook" => FieldValue::Single(contact.facebook.clone()),
        "instagram" => FieldValue::Single(contact.instagram.clone()),
        "github" => FieldValue::Single(contact.github.clone()),
        "website" => FieldValue::Single(contact.website.clone()),

        // Personal
        "birthday" => match contact.birthday {
            Some(d) => FieldValue::Single(d.format("%Y-%m-%d").to_string()),
            None => FieldValue::None,
        },
        "interests" => FieldValue::List(contact.interests.clone()),
        "family" => FieldValue::List(contact.family.clone()),

        // Relationship
        "how_we_met" => FieldValue::Single(contact.how_we_met.clone()),
        "met_date" => match contact.met_date {
            Some(d) => FieldValue::Single(d.format("%Y-%m-%d").to_string()),
            None => FieldValue::None,
        },
        "introduced_by" => FieldValue::Single(contact.introduced_by.clone()),
        "relationship" => match &contact.relationship {
            Some(r) => {
                // Use serde_json to get the snake_case serialization
                let s = serde_json::to_string(r).unwrap_or_default();
                // Strip quotes from JSON string
                FieldValue::Single(s.trim_matches('"').to_string())
            }
            None => FieldValue::None,
        },
        "tags" => FieldValue::List(contact.tags.clone()),

        // CRM
        "status" => match &contact.status {
            Some(s) => {
                // Use serde_json to get the kebab-case serialization
                let serialized = serde_json::to_string(s).unwrap_or_default();
                // Strip quotes from JSON string
                FieldValue::Single(serialized.trim_matches('"').to_string())
            }
            None => FieldValue::None,
        },
        "follow_up_cadence" => FieldValue::Single(contact.follow_up_cadence.clone()),
        "last_contacted" => match contact.last_contacted {
            Some(d) => FieldValue::Single(d.format("%Y-%m-%d").to_string()),
            None => FieldValue::None,
        },
        "next_follow_up" => match contact.next_follow_up {
            Some(d) => FieldValue::Single(d.format("%Y-%m-%d").to_string()),
            None => FieldValue::None,
        },
        "priority" => match &contact.priority {
            Some(p) => {
                let s = serde_json::to_string(p).unwrap_or_default();
                FieldValue::Single(s.trim_matches('"').to_string())
            }
            None => FieldValue::None,
        },

        // Source
        "source" => FieldValue::Single(contact.source.clone()),
        "source_id" => FieldValue::Single(contact.source_id.clone()),
        "etag" => FieldValue::Single(contact.etag.clone()),

        // Unknown field
        _ => FieldValue::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::contact::{Priority, Relationship, Status};
    use chrono::NaiveDate;

    fn make_contact() -> Contact {
        Contact {
            id: "test-id".to_string(),
            name: "Jane Smith".to_string(),
            aliases: vec!["Janey".to_string()],
            pronouns: "she/her".to_string(),
            email: vec!["jane@example.com".to_string()],
            phone: vec![],
            address: vec![],
            company: "Acme Corp".to_string(),
            role: "Engineer".to_string(),
            industry: "Tech".to_string(),
            linkedin: String::new(),
            twitter: String::new(),
            facebook: String::new(),
            instagram: String::new(),
            github: String::new(),
            website: String::new(),
            birthday: Some(NaiveDate::from_ymd_opt(1990, 6, 15).unwrap()),
            interests: vec!["rust".to_string(), "hiking".to_string()],
            family: vec![],
            how_we_met: "Conference".to_string(),
            met_date: None,
            introduced_by: String::new(),
            relationship: Some(Relationship::Friend),
            tags: vec!["friend".to_string(), "engineer".to_string()],
            status: Some(Status::Dormant),
            follow_up_cadence: "monthly".to_string(),
            last_contacted: Some(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
            next_follow_up: Some(NaiveDate::from_ymd_opt(2026, 2, 15).unwrap()),
            priority: Some(Priority::High),
            source: "manual".to_string(),
            source_id: String::new(),
            etag: String::new(),
        }
    }

    // ── Parse tests ─────────────────────────────────────────────────────

    #[test]
    fn query_parse_eq_operator() {
        let q = Query::parse("status=dormant").unwrap();
        assert_eq!(q.predicates.len(), 1);
        assert_eq!(q.predicates[0].field, "status");
        assert_eq!(q.predicates[0].op, Op::Eq);
        assert_eq!(q.predicates[0].value, "dormant");
    }

    #[test]
    fn query_parse_neq_operator() {
        let q = Query::parse("status!=active").unwrap();
        assert_eq!(q.predicates.len(), 1);
        assert_eq!(q.predicates[0].op, Op::NotEq);
    }

    #[test]
    fn query_parse_contains_operator() {
        let q = Query::parse("tags~friend").unwrap();
        assert_eq!(q.predicates.len(), 1);
        assert_eq!(q.predicates[0].op, Op::Contains);
    }

    #[test]
    fn query_parse_space_separator() {
        let q = Query::parse("status=dormant tags~friend").unwrap();
        assert_eq!(q.predicates.len(), 2);
    }

    #[test]
    fn query_parse_comma_separator() {
        let q = Query::parse("status=dormant,tags~friend").unwrap();
        assert_eq!(q.predicates.len(), 2);
    }

    #[test]
    fn query_parse_empty_returns_error() {
        assert!(Query::parse("").is_err());
    }

    #[test]
    fn query_parse_no_operator_returns_error() {
        assert!(Query::parse("invalid").is_err());
    }

    // ── Matches tests ───────────────────────────────────────────────────

    #[test]
    fn query_matches_status_dormant() {
        let q = Query::parse("status=dormant").unwrap();
        assert!(q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_status_lost_touch_kebab_case() {
        let mut c = make_contact();
        c.status = Some(Status::LostTouch);
        let q = Query::parse("status=lost-touch").unwrap();
        assert!(q.matches(&c));
    }

    #[test]
    fn query_matches_tags_eq_contains_semantics() {
        // tags=friend should match if "friend" is IN the tags list
        let q = Query::parse("tags=friend").unwrap();
        assert!(q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_tags_contains_substring() {
        // tags~fri should match if any tag contains "fri" substring
        let q = Query::parse("tags~fri").unwrap();
        assert!(q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_company_case_insensitive() {
        // Use contains operator for multi-word values, or exact single-word match
        let q = Query::parse("company~acme").unwrap();
        assert!(q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_company_eq_case_insensitive() {
        // Single-word company for exact Eq test
        let mut c = make_contact();
        c.company = "Acme".to_string();
        let q = Query::parse("company=acme").unwrap();
        assert!(q.matches(&c));
    }

    #[test]
    fn query_matches_multiple_predicates_both_must_match() {
        let q = Query::parse("status=dormant tags=friend").unwrap();
        assert!(q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_multiple_predicates_one_fails() {
        let q = Query::parse("status=active tags=friend").unwrap();
        assert!(!q.matches(&make_contact()));
    }

    #[test]
    fn query_matches_neq_operator() {
        let q = Query::parse("status!=active").unwrap();
        assert!(q.matches(&make_contact())); // status is dormant, not active
    }

    #[test]
    fn query_matches_unknown_field_returns_false() {
        let q = Query::parse("nonexistent=value").unwrap();
        assert!(!q.matches(&make_contact()));
    }

    // ── get_field_value tests ───────────────────────────────────────────

    #[test]
    fn query_get_field_value_status_serde_serialization() {
        let c = make_contact();
        match get_field_value(&c, "status") {
            FieldValue::Single(s) => assert_eq!(s, "dormant"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn query_get_field_value_status_lost_touch_kebab() {
        let mut c = make_contact();
        c.status = Some(Status::LostTouch);
        match get_field_value(&c, "status") {
            FieldValue::Single(s) => assert_eq!(s, "lost-touch"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn query_get_field_value_relationship_snake_case() {
        let c = make_contact();
        match get_field_value(&c, "relationship") {
            FieldValue::Single(s) => assert_eq!(s, "friend"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn query_get_field_value_priority_snake_case() {
        let c = make_contact();
        match get_field_value(&c, "priority") {
            FieldValue::Single(s) => assert_eq!(s, "high"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn query_get_field_value_tags_returns_list() {
        let c = make_contact();
        match get_field_value(&c, "tags") {
            FieldValue::List(items) => {
                assert!(items.contains(&"friend".to_string()));
                assert!(items.contains(&"engineer".to_string()));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn query_get_field_value_birthday_formatted() {
        let c = make_contact();
        match get_field_value(&c, "birthday") {
            FieldValue::Single(s) => assert_eq!(s, "1990-06-15"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn query_get_field_value_none_for_missing_date() {
        let c = make_contact();
        match get_field_value(&c, "met_date") {
            FieldValue::None => {}
            _ => panic!("Expected None"),
        }
    }

    #[test]
    fn query_get_field_value_unknown_field() {
        let c = make_contact();
        match get_field_value(&c, "nonexistent") {
            FieldValue::None => {}
            _ => panic!("Expected None"),
        }
    }
}
