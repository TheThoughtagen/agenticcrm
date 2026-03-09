use std::fmt;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::LogResult;
use crate::store;

impl fmt::Display for LogResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Logged {} with {} -- {}",
            self.interaction_type, self.name, self.path
        )?;
        if let Some(ref next) = self.next_follow_up {
            write!(f, "\nNext follow-up: {next}")?;
        }
        Ok(())
    }
}

/// Re-export for backward compatibility (TUI references this).
pub fn next_follow_up(
    from_date: chrono::NaiveDate,
    cadence: &str,
) -> anyhow::Result<Option<chrono::NaiveDate>> {
    ops::contact::next_follow_up(from_date, cadence).map_err(|e| anyhow::anyhow!(e))
}

pub fn run(
    name: &str,
    interaction_type: &str,
    summary: &str,
    notes: Option<&str>,
    output_format: &OutputFormat,
) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::contact::log_interaction(&root, name, interaction_type, summary, notes)?;
    format::output(&result, output_format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

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
        assert!(err.contains("monthly"));
    }
}
