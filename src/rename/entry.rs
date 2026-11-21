//! The registry row of a renamed repository.
use crate::registry::file::Entry;
use std::path::Path;

/// `entries` with the row named `old` renamed to `new` at `path`; `None` when no row has that name.
pub fn renamed(entries: &[Entry], old: &str, new: &str, path: &Path) -> Option<Vec<Entry>> {
    let at = entries.iter().position(|e| e.name == old)?;
    let mut out = entries.to_vec();
    out[at] = Entry {
        name: new.into(),
        path: path.into(),
    };
    Some(out)
}

/// Like [`renamed`], for a rename that failed part-way: the row follows the
/// folder only once the old one is gone and the new one is there, so the
/// registry says where the project really is, whatever step failed.
pub fn reconciled(
    entries: &[Entry],
    old: &str,
    new: &str,
    new_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<Vec<Entry>> {
    let old_dir = entries.iter().find(|e| e.name == old)?.path.clone();
    (!exists(&old_dir) && exists(new_dir))
        .then(|| renamed(entries, old, new, new_dir))
        .flatten()
}

#[cfg(test)]
mod tests {
    include!("entry_tests.rs");
}
