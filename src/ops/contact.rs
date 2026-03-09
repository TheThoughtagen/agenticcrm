use serde::Serialize;

use super::OpsError;

// Re-export chrono types needed by next_follow_up callers
pub use chrono::NaiveDate;
use chrono::{Duration, Months};

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
    pub contact: crate::models::Contact,
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

// ── Business logic functions (stubs for Task 1, filled in Task 2) ───────────

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
