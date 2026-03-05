use anyhow::Result;
use chrono::Local;
use colored::Colorize;

use crate::store;

pub fn run() -> Result<()> {
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
        println!("No follow-ups due. You're all caught up.");
        return Ok(());
    }

    for cf in &due {
        let c = &cf.contact;
        let follow_up = c.next_follow_up.unwrap();
        let last = c
            .last_contacted
            .map(|d| d.to_string())
            .unwrap_or_else(|| "never".to_string());
        let overdue_days = (today - follow_up).num_days();

        let date_str = if overdue_days > 0 {
            format!("{follow_up} ({overdue_days}d overdue)").red().to_string()
        } else {
            follow_up.to_string()
        };

        println!(
            "{} {} — last: {}",
            date_str,
            c.name.bold(),
            last.dimmed()
        );
    }

    println!("\n{} due", due.len());
    Ok(())
}
