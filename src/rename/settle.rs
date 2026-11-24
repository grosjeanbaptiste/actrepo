//! How `actrepo rename` ends: the registry says where the project really is,
//! whichever step failed.
use super::entry::{reconciled, renamed};
use crate::registry::file::Entry;
use anyhow::{Context, Result};
use std::path::Path;

/// The end of a rename: the registry row follows the folder, on success and
/// on a failure part-way. `Ok` carries the warnings to say; `Err` the failure,
/// or the registry write that failed on top of it.
pub fn settle(
    entries: &[Entry],
    old: &str,
    new: &str,
    new_dir: &Path,
    executed: Result<()>,
    exists: &dyn Fn(&Path) -> bool,
    write: &mut dyn FnMut(&[Entry]) -> Result<()>,
) -> Result<Vec<String>> {
    match executed {
        Ok(()) => match renamed(entries, old, new, new_dir) {
            Some(rows) => {
                write(&rows).context("renamed everywhere, but the registry was not updated")?;
                Ok(vec![])
            }
            None => Ok(vec![format!("{old} was not in the registry")]),
        },
        Err(failure) => {
            if let Some(rows) = reconciled(entries, old, new, new_dir, exists) {
                write(&rows).with_context(|| {
                    format!("the rename failed ({failure:#}) and the registry was not updated")
                })?;
            }
            Err(failure)
        }
    }
}

#[cfg(test)]
mod tests {
    include!("settle_tests.rs");
}
