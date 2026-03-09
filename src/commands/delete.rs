use std::fmt;

use dialoguer::Confirm;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::DeleteResult;
use crate::store;

impl fmt::Display for DeleteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.deleted {
            write!(f, "Deleted {} ({})", self.name, self.path)
        } else {
            write!(f, "Cancelled deletion of {}", self.name)
        }
    }
}

pub fn run(name: &str, yes: bool, format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let target = ops::contact::find_delete_target(&root, name)?;

    let confirmed = if yes {
        true
    } else {
        Confirm::new()
            .with_prompt(format!(
                "Delete {}? This cannot be undone.",
                target.name
            ))
            .default(false)
            .interact()?
    };

    if confirmed {
        let result = ops::contact::confirm_delete(&root, name)?;
        format::output(&result, format)
    } else {
        let result = DeleteResult {
            name: target.name,
            path: target.path,
            deleted: false,
        };
        format::output(&result, format)
    }
}
