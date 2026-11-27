//! The linked worktrees of a repository, listed before a move and repaired after it.
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The linked worktrees in `git worktree list --porcelain` output: every
/// `worktree` line after the first, which is the main one.
pub fn linked(porcelain: &str) -> Vec<PathBuf> {
    porcelain
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .skip(1)
        .map(PathBuf::from)
        .collect()
}

fn git(dir: &Path, args: &[&str], paths: &[PathBuf]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .args(paths)
        .output()
        .context("cannot start git")?;
    if !out.status.success() {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The linked worktrees of the repository at `dir`.
pub fn list(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(linked(&git(dir, &["worktree", "list", "--porcelain"], &[])?))
}

/// Point the linked worktrees back at the repository, now at `dir`.
pub fn repair(dir: &Path, worktrees: &[PathBuf]) -> Result<()> {
    git(dir, &["worktree", "repair"], worktrees).map(|_| ())
}

#[cfg(test)]
mod tests {
    include!("worktrees_tests.rs");
}
