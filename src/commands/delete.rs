use std::fmt;

use anyhow::Result;
use dialoguer::Confirm;
use serde::Serialize;

use crate::format::{self, OutputFormat};
use crate::store;

#[derive(Serialize)]
pub struct DeleteResult {
    pub name: String,
    pub path: String,
    pub deleted: bool,
}

impl fmt::Display for DeleteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.deleted {
            write!(f, "Deleted {} ({})", self.name, self.path)
        } else {
            write!(f, "Cancelled deletion of {}", self.name)
        }
    }
}

pub fn run(name: &str, yes: bool, format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&root)?;
    let cf = store::find_single_contact(contacts, name)?;

    let confirmed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Delete {}? This cannot be undone.",
                cf.contact.name
            ))
            .default(false)
            .interact()?
    };

    if confirmed {
        std::fs::remove_file(&cf.path)?;

        let result = DeleteResult {
            name: cf.contact.name,
            path: cf.path.display().to_string(),
            deleted: true,
        };
        format::output(&result, format)
    } else {
        let result = DeleteResult {
            name: cf.contact.name,
            path: cf.path.display().to_string(),
            deleted: false,
        };
        format::output(&result, format)
    }
}
