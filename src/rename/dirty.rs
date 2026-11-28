//! Refuse a rename that would rewrite a tree holding uncommitted changes.
//! The rewrite is a blind word replacement under every root: on a dirty tree it mixes into
//! unsaved work and `git restore` can no longer undo it (dotfiles, 2026-11-27).
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Trees of `root` (itself and its linked worktrees) with uncommitted changes.
pub fn git_dirty_trees(root: &Path) -> Result<Vec<PathBuf>> {
    let trees = git(root, &["worktree", "list", "--porcelain"]);
    let Ok(trees) = trees else {
        // not a git checkout: nothing to lose to a rewrite, and nothing to compare against
        return Ok(vec![]);
    };
    let mut dirty = vec![];
    for tree in trees.lines().filter_map(|l| l.strip_prefix("worktree ")) {
        if !git(Path::new(tree), &["status", "--porcelain"])?
            .trim()
            .is_empty()
        {
            dirty.push(PathBuf::from(tree));
        }
    }
    Ok(dirty)
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
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

/// Fails, naming every dirty tree, unless `allow` is set.
pub fn ensure_clean(
    roots: &[PathBuf],
    dirty_of: &dyn Fn(&Path) -> Result<Vec<PathBuf>>,
    allow: bool,
) -> Result<()> {
    if allow {
        return Ok(());
    }
    let mut dirty = vec![];
    for root in roots {
        dirty.extend(dirty_of(root)?);
    }
    if dirty.is_empty() {
        return Ok(());
    }
    let list: Vec<_> = dirty.iter().map(|p| p.display().to_string()).collect();
    bail!(
        "refusing to rewrite trees with uncommitted changes (commit them, or pass --allow-dirty):\n  {}",
        list.join("\n  ")
    )
}

#[cfg(test)]
mod tests {
    include!("dirty_tests.rs");
}
