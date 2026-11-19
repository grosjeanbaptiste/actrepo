//! `$DOTFILES_STATE_ROOT/projects.txt`: one `name|path` per line.
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
}

impl Entry {
    /// Case-insensitive like the shell `sort` was; exact name then path break ties.
    fn sort_key(&self) -> (String, String, PathBuf) {
        (
            self.name.to_lowercase(),
            self.name.clone(),
            self.path.clone(),
        )
    }
}

pub fn sorted(entries: &[Entry]) -> Vec<Entry> {
    let mut v = entries.to_vec();
    v.sort_by_key(Entry::sort_key);
    v
}

/// The machine state root: `DOTFILES_STATE_ROOT`, else `~/machine`.
pub fn state_root(env: impl Fn(&str) -> Option<String>) -> Result<PathBuf> {
    if let Some(root) = env("DOTFILES_STATE_ROOT").filter(|r| !r.is_empty()) {
        return Ok(PathBuf::from(root));
    }
    let home = env("HOME")
        .filter(|h| !h.is_empty())
        .context("HOME is not set")?;
    Ok(PathBuf::from(home).join("machine"))
}

/// `<state root>/projects.txt`.
pub fn path(env: impl Fn(&str) -> Option<String>) -> Result<PathBuf> {
    Ok(state_root(env)?.join("projects.txt"))
}

/// Entries of the registry text, comments and blank lines dropped; a row
/// that is not `name|path` is an error naming its line.
pub fn parse(text: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        match line.split_once('|') {
            Some((name, path)) if !name.trim().is_empty() && !path.trim().is_empty() => {
                entries.push(Entry {
                    name: name.into(),
                    path: path.into(),
                });
            }
            _ => bail!(
                "projects.txt line {}: expected name|path, got {line:?}",
                i + 1
            ),
        }
    }
    Ok(entries)
}

/// A missing file is an empty registry; an unreadable one is an error.
pub fn read(file: &Path) -> Result<Vec<Entry>> {
    match fs::read_to_string(file) {
        Ok(text) => parse(&text).with_context(|| file.display().to_string()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).with_context(|| format!("reading {}", file.display())),
    }
}

pub fn find(entries: &[Entry], name: &str) -> Option<PathBuf> {
    entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.path.clone())
}

/// What `add` did.
#[derive(Debug, PartialEq, Eq)]
pub enum Added {
    New,
    AlreadyListed,
}

/// Append `name|path` to the registry. A row already there is reported, a name
/// listed at another path is an error, and a name or path that would break the
/// one-row-per-line format is refused. The registry must already exist.
pub fn add(file: &Path, name: &str, path: &Path) -> Result<Added> {
    let shown = path.display().to_string();
    if name.trim().is_empty()
        || [name, shown.as_str()]
            .iter()
            .any(|t| t.contains(['|', '\n', '\r']))
    {
        bail!("cannot register {name:?} at {shown:?}: empty or holds | or a line break");
    }
    let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    if let Some(at) = find(&parse(&text)?, name) {
        if at == path {
            return Ok(Added::AlreadyListed);
        }
        bail!(
            "{name} is already registered at {}, not {shown}",
            at.display()
        );
    }
    let glue = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    fs::write(file, format!("{text}{glue}{name}|{shown}\n"))
        .with_context(|| format!("writing {}", file.display()))?;
    Ok(Added::New)
}

/// The `list` report: sorted, `(missing)` flags a path gone from disk.
pub fn list(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in sorted(entries) {
        let flag = if e.path.is_dir() { "" } else { " (missing)" };
        out.push_str(&format!("{:<30} {}{flag}\n", e.name, e.path.display()));
    }
    out
}

#[cfg(test)]
mod tests {
    include!("file_tests.rs");
}
