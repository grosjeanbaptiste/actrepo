//! What a project must be before `repo install` compiles it: registered, Cargo, committed, clean.
use crate::registry::file::Entry;
use crate::registry::file::find;
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::path::{Path, PathBuf};

fn readable(root: &Path, file: &str) -> Result<()> {
    match File::open(root.join(file)) {
        Ok(_) => Ok(()),
        Err(_) => bail!("{file} not found in {}", root.display()),
    }
}

/// The canonical path of project `name`, or why it cannot be installed.
pub fn project(entries: &[Entry], name: &str, probe: &dyn Exec) -> Result<PathBuf> {
    let Some(path) = find(entries, name) else {
        bail!("Unknown project: {name} (run repo list)");
    };
    if !path.is_dir() {
        bail!("registered path is missing: {}", path.display());
    }
    let root = path
        .canonicalize()
        .with_context(|| format!("cannot resolve project path: {}", path.display()))?;
    readable(&root, "Cargo.toml")?;
    readable(&root, "Cargo.lock")?;
    let git = |args: &[&str]| probe.run("git", args, Some(&root));
    if git(&["rev-parse", "--verify", "HEAD"]).is_err() {
        bail!(
            "project is not a committed Git repository: {}",
            root.display()
        );
    }
    // every untracked file on its own line, so a stray one is never hidden inside a folder entry
    let status = git(&[
        "--no-optional-locks",
        "status",
        "--porcelain",
        "--untracked-files=all",
    ])
    .with_context(|| format!("cannot read Git status for {}", root.display()))?;
    let listing = git(&["worktree", "list", "--porcelain"])
        .with_context(|| format!("cannot list the worktrees of {}", root.display()))?;
    if has_changes(&status, &listing, &root) {
        bail!("project has uncommitted changes: {}", root.display());
    }
    probe.run("cargo", &["--version"], None).map_err(|_| {
        anyhow::anyhow!("cargo is unavailable; install Rust through dotfiles first")
    })?;
    Ok(root)
}

/// True when `status` holds a real change: a linked worktree kept inside the project
/// (`.worktrees/x`) shows as an untracked folder, and is not one.
fn has_changes(status: &str, listing: &str, root: &Path) -> bool {
    let worktrees: Vec<PathBuf> = listing
        .lines()
        .filter_map(|l| l.strip_prefix("worktree "))
        .skip(1)
        .map(PathBuf::from)
        .collect();
    status
        .lines()
        .filter(|l| !l.trim().is_empty())
        .any(|l| match l.strip_prefix("?? ") {
            Some(path) => !worktrees.contains(&root.join(path.trim_end_matches('/'))),
            None => true,
        })
}

#[cfg(test)]
mod tests {
    include!("check_tests.rs");
}
