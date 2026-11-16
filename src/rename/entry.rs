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

#[cfg(test)]
mod tests {
    include!("entry_tests.rs");
}
