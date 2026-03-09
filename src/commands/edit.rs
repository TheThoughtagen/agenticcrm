use std::fmt;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::EditResult;
use crate::store;

impl fmt::Display for EditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Updated {} (fields: {}) -- {}",
            self.name,
            self.updated_fields.join(", "),
            self.path
        )
    }
}

pub fn run(name: &str, sets: &[String], format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::contact::edit(&root, name, sets)?;
    format::output(&result, format)
}
