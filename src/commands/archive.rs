use std::fmt;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::ArchiveResult;
use crate::store;

impl fmt::Display for ArchiveResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({} -> {})",
            self.action, self.name, self.from_path, self.to_path
        )
    }
}

pub fn run_archive(name: &str, format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::contact::archive(&root, name)?;
    format::output(&result, format)
}

pub fn run_unarchive(name: &str, format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::contact::unarchive(&root, name)?;
    format::output(&result, format)
}
