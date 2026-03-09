use std::fmt;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::contact::AddResult;
use crate::store;

impl fmt::Display for AddResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Created: {}", self.path)
    }
}

pub fn run(name: &str, output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::contact::add(&root, name)?;
    format::output(&result, output_format)
}
