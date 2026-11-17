//! `repo worktree list`: the worktrees of the repository in `dir`, read from `git worktree list --porcelain`.
use crate::scaffold::setup::Exec;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

/// A live agent session working in a worktree.
#[derive(Debug, PartialEq, Eq)]
pub struct Owner {
    pub id: String,
    pub agent: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Worktree {
    pub path: String,
    /// `None` when detached.
    pub branch: Option<String>,
    /// The first block of git's listing.
    pub main: bool,
    pub detached: bool,
    /// Its folder is gone.
    pub prunable: bool,
    /// `None` when owners were not asked for.
    pub owners: Option<Vec<Owner>>,
}

pub fn parse(porcelain: &str) -> Vec<Worktree> {
    porcelain
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .enumerate()
        .filter_map(|(i, block)| {
            let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(key));
            let has = |key: &str| block.lines().any(|l| l.starts_with(key));
            Some(Worktree {
                path: field("worktree ")?.to_string(),
                branch: field("branch refs/heads/").map(str::to_string),
                main: i == 0,
                detached: has("detached"),
                prunable: has("prunable"),
                owners: None,
            })
        })
        .collect()
}

pub fn list(probe: &impl Exec, dir: &Path) -> Result<Vec<Worktree>> {
    Ok(parse(&probe.run(
        "git",
        &["worktree", "list", "--porcelain"],
        Some(dir),
    )?))
}

pub fn text(all: &[Worktree]) -> String {
    all.iter()
        .map(|w| {
            let at = w.branch.as_deref().unwrap_or("(detached)");
            let note = if w.prunable { "  (folder gone)" } else { "" };
            let owners: String = w
                .owners
                .iter()
                .flatten()
                .map(|o| format!("    owner: {} {}\n", o.agent, o.id))
                .collect();
            format!("{}  {at}{note}\n{owners}", w.path)
        })
        .collect()
}

pub fn json(all: &[Worktree]) -> Value {
    Value::Array(
        all.iter()
            .map(|w| {
                let mut row = json!({"path": w.path, "branch": w.branch, "main": w.main, "detached": w.detached, "prunable": w.prunable});
                if let Some(owners) = &w.owners {
                    let owners: Vec<Value> = owners.iter().map(|o| json!({"id": o.id, "agent": o.agent})).collect();
                    row["owners"] = Value::Array(owners);
                }
                row
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    include!("list_tests.rs");
}
