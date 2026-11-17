//! JSON views of `list` and `status`, for tools that read `repo` instead of parsing its text.
use super::file::{Entry, sorted};
use super::status::{Status, git_status};
use crate::scaffold::setup::Exec;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::path::Path;

pub fn list(entries: &[Entry]) -> Value {
    let rows: Vec<Value> = sorted(entries)
        .iter()
        .map(|e| json!({"name": e.name, "path": e.path.display().to_string()}))
        .collect();
    Value::Array(rows)
}

fn view(name: &str, path: &Path, s: &Status) -> Value {
    json!({
        "name": name,
        "path": path.display().to_string(),
        "branch": s.branch,
        "ahead": s.ahead,
        "behind": s.behind,
        "dirty": s.dirty,
        "remote": (s.remote != "none").then_some(&s.remote), // "none" is the text report's word for no origin
    })
}

pub fn status_one(exec: &impl Exec, name: &str, path: &Path) -> Result<Value> {
    let Some(s) = git_status(exec, path)? else {
        bail!(
            "{name} ({}): not a git repository (or path missing)",
            path.display()
        );
    };
    Ok(view(name, path, &s))
}

/// Same rows as the text table: existing repositories only; one that cannot be read is a warning.
pub fn status_all(exec: &impl Exec, entries: &[Entry], warnings: &mut Vec<String>) -> Value {
    let mut rows = Vec::new();
    for e in sorted(entries).iter().filter(|e| e.path.is_dir()) {
        match git_status(exec, &e.path) {
            Ok(Some(s)) => rows.push(view(&e.name, &e.path, &s)),
            Ok(None) => {} // not a repository any more: not a row
            Err(err) => warnings.push(format!("{} ({}): {err:#}", e.name, e.path.display())),
        }
    }
    Value::Array(rows)
}
