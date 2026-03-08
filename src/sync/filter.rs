use crate::models::contact::{Contact, Status};

/// Filter for selecting which contacts to sync.
/// When empty (default), matches all contacts.
#[derive(Debug, Default, Clone)]
pub struct SyncFilter {
    pub tags: Vec<String>,
    pub statuses: Vec<String>,
}

impl SyncFilter {
    /// Returns true when no filters are configured (matches everything).
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.statuses.is_empty()
    }

    /// Check if a contact matches this filter.
    /// Empty filter matches all contacts.
    /// Tags: contact must have at least one matching tag.
    /// Statuses: contact status must match one of the filter statuses (kebab-case).
    pub fn matches(&self, contact: &Contact) -> bool {
        let tag_ok =
            self.tags.is_empty() || contact.tags.iter().any(|t| self.tags.contains(t));

        let status_ok = self.statuses.is_empty()
            || contact.status.as_ref().map_or(false, |s| {
                let status_str = match s {
                    Status::Active => "active",
                    Status::Dormant => "dormant",
                    Status::LostTouch => "lost-touch",
                    Status::Archived => "archived",
                };
                self.statuses.iter().any(|fs| fs == status_str)
            });

        tag_ok && status_ok
    }

    /// Build a SyncFilter from config values and CLI overrides.
    /// CLI values replace (not union) config values when non-empty.
    pub fn from_config_and_cli(
        config_tags: Vec<String>,
        config_statuses: Vec<String>,
        cli_tags: Vec<String>,
        cli_statuses: Vec<String>,
    ) -> Self {
        SyncFilter {
            tags: if cli_tags.is_empty() {
                config_tags
            } else {
                cli_tags
            },
            statuses: if cli_statuses.is_empty() {
                config_statuses
            } else {
                cli_statuses
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_contact(tags: &[&str], status: Option<Status>) -> Contact {
        Contact {
            id: "test-id".to_string(),
            name: "Test Contact".to_string(),
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
            tags: tags.iter().map(|s| s.to_string()).collect(),
            status,
            follow_up_cadence: String::new(),
            last_contacted: None,
            next_follow_up: None,
            priority: None,
            source: String::new(),
            source_id: String::new(),
            etag: String::new(),
        }
    }

    #[test]
    fn test_default_is_empty() {
        let f = SyncFilter::default();
        assert!(f.is_empty());
    }

    #[test]
    fn test_empty_filter_matches_all() {
        let f = SyncFilter::default();
        let c = make_contact(&["work"], Some(Status::Active));
        assert!(f.matches(&c));
    }

    #[test]
    fn test_empty_filter_matches_contact_with_no_status() {
        let f = SyncFilter::default();
        let c = make_contact(&[], None);
        assert!(f.matches(&c));
    }

    #[test]
    fn test_tag_filter_matches() {
        let f = SyncFilter {
            tags: vec!["work".to_string()],
            statuses: vec![],
        };
        let c = make_contact(&["work", "friend"], Some(Status::Active));
        assert!(f.matches(&c));
    }

    #[test]
    fn test_tag_filter_no_match() {
        let f = SyncFilter {
            tags: vec!["work".to_string()],
            statuses: vec![],
        };
        let c = make_contact(&["personal"], Some(Status::Active));
        assert!(!f.matches(&c));
    }

    #[test]
    fn test_status_filter_matches() {
        let f = SyncFilter {
            tags: vec![],
            statuses: vec!["active".to_string()],
        };
        let c = make_contact(&[], Some(Status::Active));
        assert!(f.matches(&c));
    }

    #[test]
    fn test_status_filter_lost_touch_kebab_case() {
        let f = SyncFilter {
            tags: vec![],
            statuses: vec!["lost-touch".to_string()],
        };
        let c = make_contact(&[], Some(Status::LostTouch));
        assert!(f.matches(&c));
    }

    #[test]
    fn test_status_filter_no_match_none_status() {
        let f = SyncFilter {
            tags: vec![],
            statuses: vec!["active".to_string()],
        };
        let c = make_contact(&[], None);
        assert!(!f.matches(&c));
    }

    #[test]
    fn test_combined_tag_and_status_both_must_match() {
        let f = SyncFilter {
            tags: vec!["work".to_string()],
            statuses: vec!["active".to_string()],
        };
        // Has matching tag but wrong status
        let c = make_contact(&["work"], Some(Status::Dormant));
        assert!(!f.matches(&c));
    }

    #[test]
    fn test_from_config_and_cli_uses_cli_when_provided() {
        let f = SyncFilter::from_config_and_cli(
            vec!["config-tag".to_string()],
            vec!["active".to_string()],
            vec!["cli-tag".to_string()],
            vec!["dormant".to_string()],
        );
        assert_eq!(f.tags, vec!["cli-tag"]);
        assert_eq!(f.statuses, vec!["dormant"]);
    }

    #[test]
    fn test_from_config_and_cli_falls_back_to_config() {
        let f = SyncFilter::from_config_and_cli(
            vec!["config-tag".to_string()],
            vec!["active".to_string()],
            vec![],
            vec![],
        );
        assert_eq!(f.tags, vec!["config-tag"]);
        assert_eq!(f.statuses, vec!["active"]);
    }
}
