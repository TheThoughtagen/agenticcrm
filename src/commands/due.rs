use std::fmt;

use colored::Colorize;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::DueContact;
use crate::store;

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

pub fn run(output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let results = ops::contact::due(&root)?;

    if results.is_empty() {
        match output_format {
            OutputFormat::Human => println!("No follow-ups due. You're all caught up."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    format::output_list(&results, output_format, "due")
}
