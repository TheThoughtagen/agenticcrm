use std::fmt;

use anyhow::{Context, Result, bail};
use chrono::{Duration, Local, Months, NaiveDate};
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::store;

#[derive(Serialize)]
pub struct LogResult {
    pub name: String,
    pub interaction_type: String,
    pub summary: String,
    pub path: String,
    pub last_contacted: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_follow_up: Option<String>,
}

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

/// Calculate the next follow-up date from a given date and cadence string.
/// Returns Ok(None) if cadence is empty.
/// Returns Err if cadence is unknown.
pub fn next_follow_up(from_date: NaiveDate, cadence: &str) -> Result<Option<NaiveDate>> {
    let cadence = cadence.trim();
    if cadence.is_empty() {
        return Ok(None);
    }

    let next = match cadence {
        "weekly" => from_date + Duration::days(7),
        "biweekly" | "bi-weekly" => from_date + Duration::days(14),
        "monthly" => from_date
            .checked_add_months(Months::new(1))
            .context("Date overflow adding 1 month")?,
        "quarterly" => from_date
            .checked_add_months(Months::new(3))
            .context("Date overflow adding 3 months")?,
        "yearly" | "annually" => from_date
            .checked_add_months(Months::new(12))
            .context("Date overflow adding 12 months")?,
        _ => bail!(
            "Unknown cadence: '{cadence}'. Supported: weekly, biweekly, monthly, quarterly, yearly"
        ),
    };

    Ok(Some(next))
}

pub fn run(
    name: &str,
    interaction_type: &str,
    summary: &str,
    notes: Option<&str>,
    output_format: &OutputFormat,
) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let mut cf = store::find_single_contact(contacts, name)?;
    let today = Local::now().date_naive();

    // Build the interaction entry
    let mut entry = format!("\n### {today} | {interaction_type} | {summary}\n");
    if let Some(n) = notes {
        entry.push('\n');
        entry.push_str(n);
        entry.push('\n');
    }

    // Insert after "## Interaction Log" heading
    if let Some(pos) = cf.body.find("## Interaction Log") {
        let insert_at = cf.body[pos..]
            .find('\n')
            .map(|i| pos + i + 1)
            .unwrap_or(cf.body.len());
        cf.body.insert_str(insert_at, &entry);
    } else {
        cf.body.push_str("\n## Interaction Log\n");
        cf.body.push_str(&entry);
    }

    // Update last_contacted via raw frontmatter editor
    cf.raw_frontmatter =
        frontmatter::update_field(&cf.raw_frontmatter, "last_contacted", &today.to_string());

    // Calculate and update next_follow_up if cadence is set
    let next_fu = if !cf.contact.follow_up_cadence.is_empty() {
        let next = next_follow_up(today, &cf.contact.follow_up_cadence)?;
        if let Some(date) = next {
            cf.raw_frontmatter = frontmatter::update_field(
                &cf.raw_frontmatter,
                "next_follow_up",
                &date.to_string(),
            );
        }
        next.map(|d| d.to_string())
    } else {
        None
    };

    // Write directly to existing file path (preserves comments via raw frontmatter)
    let content = format!("---\n{}---\n\n{}", cf.raw_frontmatter, cf.body);
    std::fs::write(&cf.path, &content)
        .with_context(|| format!("Failed to write {}", cf.path.display()))?;

    let result = LogResult {
        name: cf.contact.name.clone(),
        interaction_type: interaction_type.to_string(),
        summary: summary.to_string(),
        path: cf.path.display().to_string(),
        last_contacted: today.to_string(),
        next_follow_up: next_fu,
    };

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
        assert!(err.contains("weekly"));
        assert!(err.contains("monthly"));
    }
}
