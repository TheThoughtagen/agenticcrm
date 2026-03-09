use std::fmt;

use anyhow::Result;
use colored::Colorize;

use crate::format::{self, OutputFormat};
use crate::ops::contact::ContactDetail;
use crate::store;

impl fmt::Display for ContactDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = &self.contact;
        writeln!(f, "{}", c.name.bold())?;

        if !c.company.is_empty() || !c.role.is_empty() {
            writeln!(f, "{} @ {}", c.role, c.company)?;
        }

        if !c.email.is_empty() {
            writeln!(f, "Email: {}", c.email.join(", "))?;
        }
        if !c.phone.is_empty() {
            writeln!(f, "Phone: {}", c.phone.join(", "))?;
        }

        let socials: Vec<(&str, &str)> = vec![
            ("LinkedIn", &c.linkedin),
            ("Twitter", &c.twitter),
            ("GitHub", &c.github),
            ("Website", &c.website),
        ];
        for (label, val) in socials {
            if !val.is_empty() {
                writeln!(f, "{label}: {val}")?;
            }
        }

        if !c.tags.is_empty() {
            writeln!(f, "Tags: {}", c.tags.join(", ").dimmed())?;
        }
        if !c.how_we_met.is_empty() {
            writeln!(f, "Met: {}", c.how_we_met)?;
        }
        if let Some(last) = c.last_contacted {
            writeln!(f, "Last contacted: {last}")?;
        }
        if let Some(next) = c.next_follow_up {
            writeln!(f, "Next follow-up: {next}")?;
        }

        if !self.body.is_empty() {
            writeln!(f)?;
            write!(f, "{}", self.body)?;
        }

        Ok(())
    }
}

pub fn run(name: &str, output_format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let cf = store::find_single_contact(contacts, name)?;
    let detail = ContactDetail {
        contact: cf.contact,
        body: cf.body,
    };

    format::output(&detail, output_format)
}
