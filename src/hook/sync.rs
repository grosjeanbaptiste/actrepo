//! What runs after a commit: skip what must never be pushed, fast-forward the
//! branch from its upstream, push it. Never a force, never `--no-verify`.
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Skip {
    Detached,
    DefaultBranch(String),
    NoOrigin,
    Forbidden(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Skipped(Skip),
    Synced { pulled: bool },
}

const DEFAULT_BRANCHES: [&str; 2] = ["main", "master"];

fn git(exec: &impl Exec, dir: &Path, args: &[&str]) -> Result<String> {
    Ok(exec.run("git", args, Some(dir))?.trim().to_owned())
}

/// Why this repository or branch is left alone, if it is.
fn skip_reason(exec: &impl Exec, dir: &Path, branch: &str) -> Result<Option<Skip>> {
    if branch == "HEAD" {
        return Ok(Some(Skip::Detached));
    }
    if DEFAULT_BRANCHES.contains(&branch) {
        return Ok(Some(Skip::DefaultBranch(branch.to_owned())));
    }
    let Ok(url) = git(exec, dir, &["remote", "get-url", "origin"]) else {
        // `remote get-url` fails exactly when there is no origin: nothing to push to.
        return Ok(Some(Skip::NoOrigin));
    };
    // Off limits by the owner's standing prohibition: the emvi repositories.
    if url.to_lowercase().contains("emvi") {
        return Ok(Some(Skip::Forbidden(url)));
    }
    Ok(None)
}

/// Commits the upstream has and the branch lacks (0 without an upstream).
fn behind(exec: &impl Exec, dir: &Path) -> Result<u64> {
    // Without an upstream (first push of the branch) there is nothing to pull.
    if git(exec, dir, &["rev-parse", "--abbrev-ref", "@{u}"]).is_err() {
        return Ok(0);
    }
    let n = git(exec, dir, &["rev-list", "--count", "HEAD..@{u}"])?;
    n.parse()
        .with_context(|| format!("rev-list --count: {n:?}"))
}

/// Pulls (fast-forward only) then pushes the current branch of `dir`.
pub fn sync(exec: &impl Exec, dir: &Path) -> Result<Outcome> {
    let branch = git(exec, dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if let Some(why) = skip_reason(exec, dir, &branch)? {
        return Ok(Outcome::Skipped(why));
    }
    git(exec, dir, &["fetch", "-q", "origin"]).context("fetch failed")?;
    let pulled = behind(exec, dir)? > 0;
    if pulled {
        git(exec, dir, &["merge", "--ff-only", "@{u}"]).context("merge --ff-only failed")?;
    }
    git(exec, dir, &["push", "-q", "-u", "origin", "HEAD"]).context("push failed")?;
    Ok(Outcome::Synced { pulled })
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
