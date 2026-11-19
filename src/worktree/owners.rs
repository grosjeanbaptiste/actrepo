//! Who works in a worktree: the live `actsessions` sessions whose working directory is inside it.
//! `repo` runs the `actsessions` CLI (no crate dependency): the sessions are that repo's business.
use super::list::{Owner, Worktree};
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,
    pub agent: String,
    status: String,
    cwd: Option<String>,
}

/// Active sessions whose working directory is known.
pub fn live(exec: &impl Exec) -> Result<Vec<Session>> {
    let out = exec.run("actsessions", &["list", "--json"], None)?;
    let all: Vec<Session> =
        serde_json::from_str(&out).context("actsessions list --json: unreadable answer")?;
    Ok(all
        .into_iter()
        .filter(|s| s.status == "active" && s.cwd.is_some())
        .collect())
}

/// The real path when it exists (`/tmp` and `/private/tmp` are one folder), else the path as given.
fn norm(p: &str) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| PathBuf::from(p)) // a gone folder keeps its spelling
}

/// Sets `owners` on every worktree: each session goes to the deepest worktree containing its cwd.
pub fn assign(worktrees: &mut [Worktree], sessions: &[Session]) {
    let roots: Vec<PathBuf> = worktrees.iter().map(|w| norm(&w.path)).collect();
    for w in worktrees.iter_mut() {
        w.owners = Some(vec![]);
    }
    for s in sessions {
        let Some(cwd) = s.cwd.as_deref().map(norm) else {
            continue;
        };
        let deepest = roots
            .iter()
            .enumerate()
            .filter(|(_, r)| Path::new(&cwd).starts_with(r))
            .max_by_key(|(_, r)| r.components().count());
        if let Some((i, _)) = deepest {
            worktrees[i]
                .owners
                .get_or_insert_with(Vec::new)
                .push(Owner {
                    id: s.id.clone(),
                    agent: s.agent.clone(),
                });
        }
    }
}

#[cfg(test)]
mod tests {
    include!("owners_tests.rs");
}
