//! The Path column of a markdown table of repositories, kept equal to the registry.
//! A row is `| **name** | `~/path` ... |`; the registry says where `name` lives.
pub mod command;

use crate::registry::file::Entry;
use std::path::Path;

/// What `fix` found: the text with the paths corrected, the rows it changed
/// (`name: old -> new`) and the row names the registry does not know.
#[derive(Debug, PartialEq, Eq)]
pub struct Fixed {
    pub text: String,
    pub changed: Vec<String>,
    pub unknown: Vec<String>,
}

pub fn fix(text: &str, entries: &[Entry], home: &Path) -> Fixed {
    let mut out = Fixed {
        text: String::new(),
        changed: vec![],
        unknown: vec![],
    };
    for line in text.split_inclusive('\n') {
        out.text.push_str(&row(
            line,
            entries,
            home,
            &mut out.changed,
            &mut out.unknown,
        ));
    }
    out
}

/// `~/rel` for a path under `home`, the absolute path otherwise.
fn tilde(path: &Path, home: &Path) -> String {
    path.strip_prefix(home).map_or_else(
        |_| path.display().to_string(),
        |r| format!("~/{}", r.display()),
    )
}

/// The row with its backticked `~/` path replaced; any other line comes back as it was.
fn row(
    line: &str,
    entries: &[Entry],
    home: &Path,
    changed: &mut Vec<String>,
    unknown: &mut Vec<String>,
) -> String {
    let Some(body) = line.strip_prefix('|') else {
        return line.to_string();
    };
    let cells: Vec<&str> = body.split('|').collect();
    let name = cells[0].trim();
    let (Some(name), Some(cell)) = (
        name.strip_prefix("**").and_then(|n| n.strip_suffix("**")),
        cells.get(1),
    ) else {
        return line.to_string();
    };
    let Some(open) = cell.find('`') else {
        return line.to_string();
    };
    let Some(len) = cell[open + 1..].find('`') else {
        return line.to_string();
    };
    let old = &cell[open + 1..open + 1 + len];
    if !old.starts_with("~/") {
        return line.to_string();
    }
    let Some(entry) = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)) else {
        unknown.push(name.to_string());
        return line.to_string();
    };
    let new = tilde(&entry.path, home);
    if new == old {
        return line.to_string();
    }
    changed.push(format!("{name}: {old} -> {new}"));
    let at = 1 + cells[0].len() + 1 + open + 1;
    let mut fixed = line.to_string();
    fixed.replace_range(at..at + old.len(), &new);
    fixed
}

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
