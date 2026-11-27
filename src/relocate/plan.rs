//! What `actrepo move` will change, decided before anything is touched.
use crate::registry::file::Entry;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub struct Move {
    pub name: String,
    pub from: PathBuf,
    pub to: PathBuf,
}

/// The one row `target` names: a folder path (holds `/`) or a registered name.
fn row<'a>(entries: &'a [Entry], target: &str) -> Result<&'a Entry> {
    let by_path = target.contains('/');
    let hits: Vec<&Entry> = entries
        .iter()
        .filter(|e| if by_path { e.path == Path::new(target) } else { e.name == target })
        .collect();
    match hits.as_slice() {
        [one] => Ok(one),
        [] => bail!("{target} is not registered"),
        many => {
            let paths: Vec<String> = many.iter().map(|e| e.path.display().to_string()).collect();
            bail!("{target} names several folders, give one of: {}", paths.join(", "))
        }
    }
}

pub fn build(
    entries: &[Entry],
    target: &str,
    into: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Result<Move> {
    let entry = row(entries, target)?;
    let from = entry.path.clone();
    let Some(base) = from.file_name() else {
        bail!("{} has no folder name", from.display());
    };
    if !exists(&from) {
        bail!("{} is absent", from.display());
    }
    if into.starts_with(&from) {
        bail!("cannot move {} inside itself", from.display());
    }
    let to = into.join(base);
    if to == from {
        bail!("{} is already in {}", from.display(), into.display());
    }
    if exists(&to) {
        bail!("{} already exists", to.display());
    }
    Ok(Move { name: entry.name.clone(), from, to })
}

/// `entries` with the row of the moved folder pointing at its new path; `None` when no row is there.
pub fn moved(entries: &[Entry], mv: &Move) -> Option<Vec<Entry>> {
    let at = entries.iter().position(|e| e.name == mv.name && e.path == mv.from)?;
    let mut out = entries.to_vec();
    out[at].path = mv.to.clone();
    Some(out)
}

#[cfg(test)]
mod tests {
    include!("plan_tests.rs");
}
