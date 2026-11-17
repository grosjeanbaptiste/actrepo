//! `repo worktree list`: the worktrees of the repository in `dir`, read from `git worktree list --porcelain`.
use crate::scaffold::setup::Exec;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

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
            format!("{}  {at}{note}\n", w.path)
        })
        .collect()
}

pub fn json(all: &[Worktree]) -> Value {
    Value::Array(
        all.iter()
            .map(|w| json!({"path": w.path, "branch": w.branch, "main": w.main, "detached": w.detached, "prunable": w.prunable}))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    include!("list_tests.rs");
}
