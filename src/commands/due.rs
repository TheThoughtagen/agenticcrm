use std::fmt;

use anyhow::Result;
use chrono::Local;
use colored::Colorize;
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::store;

#[derive(Serialize)]
pub struct DueContact {
    pub name: String,
    pub next_follow_up: String,
    pub last_contacted: String,
    pub overdue_days: i64,
}

impl fmt::Display for DueContact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_str = if self.overdue_days > 0 {
            format!("{} ({}d overdue)", self.next_follow_up, self.overdue_days)
                .red()
                .to_string()
        } else {
            self.next_follow_up.clone()
        };

        write!(
            f,
            "{} {} -- last: {}",
            date_str,
            self.name.bold(),
            self.last_contacted.dimmed()
        )
    }
}

pub fn run(output_format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let today = Local::now().date_naive();

    let mut due: Vec<_> = contacts
        .into_iter()
        .filter(|cf| {
            cf.contact
                .next_follow_up
                .is_some_and(|d| d <= today)
        })
        .collect();

    due.sort_by_key(|cf| cf.contact.next_follow_up);

    if due.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No follow-ups due. You're all caught up."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    let results: Vec<DueContact> = due
        .iter()
        .map(|cf| {
            let c = &cf.contact;
            let follow_up = c.next_follow_up.unwrap();
            DueContact {
                name: c.name.clone(),
                next_follow_up: follow_up.to_string(),
                last_contacted: c
                    .last_contacted
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "never".to_string()),
                overdue_days: (today - follow_up).num_days(),
            }
        })
        .collect();

    format::output_list(&results, output_format, "due")
}
