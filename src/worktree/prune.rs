//! Remove each linked worktree whose branch was pushed and is now contained in
//! `origin/<base>`, then the branch. Nothing is forced: a worktree with local
//! changes, an unpushed branch and the worktree `dir` itself are left alone.
use crate::pr::after::has_changes;
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Linked {
    path: PathBuf,
    branch: String,
}

/// Linked worktrees on a branch other than `base`; the main worktree comes first in the listing and is skipped.
fn linked(probe: &impl Exec, dir: &Path, base: &str) -> Result<Vec<Linked>> {
    let list = probe.run("git", &["worktree", "list", "--porcelain"], Some(dir))?;
    let here = probe.run("git", &["rev-parse", "--show-toplevel"], Some(dir))?;
    let mut found = Vec::new();
    for block in list.split("\n\n").skip(1) {
        let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(key));
        if block.lines().any(|l| l.starts_with("prunable")) {
            continue; // its folder is gone: `git worktree prune` forgets it, nothing to remove
        }
        let (Some(path), Some(branch)) = (field("worktree "), field("branch refs/heads/")) else {
            continue; // detached or bare: no branch to judge
        };
        if branch != base && Path::new(path) != Path::new(here.trim()) {
            found.push(Linked {
                path: PathBuf::from(path),
                branch: branch.to_string(),
            });
        }
    }
    Ok(found)
}

/// Pushed once (an upstream is configured), contained in the base, and not just the base itself.
fn merged(probe: &impl Exec, dir: &Path, branch: &str, base: &str) -> bool {
    let run = |args: &[&str]| probe.run("git", args, Some(dir));
    let (head, remote) = (format!("refs/heads/{branch}"), format!("origin/{base}"));
    // an Err from these is their answer (no upstream, not an ancestor), not a failure
    run(&["config", "--get", &format!("branch.{branch}.remote")]).is_ok()
        && run(&["merge-base", "--is-ancestor", &head, &remote]).is_ok()
        && run(&["rev-parse", &head]).ok() != run(&["rev-parse", &remote]).ok()
}

/// Returns how many worktrees were (or, with `dry_run`, would be) removed.
pub fn prune(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    dry_run: bool,
    out: &mut impl Write,
) -> Result<usize> {
    act.run("git", &["fetch", "-p"], Some(dir))
        .context("git fetch -p")?;
    let mut removed = 0;
    for w in linked(probe, dir, base)? {
        if !merged(probe, dir, &w.branch, base) {
            continue;
        }
        let p = w.path.display().to_string();
        if has_changes(probe, &w.path)? {
            writeln!(
                out,
                "kept {p}: {} is merged but the worktree has local changes",
                w.branch
            )?;
        } else if dry_run {
            writeln!(out, "would remove {p} and {}", w.branch)?;
            removed += 1;
        } else {
            act.run("git", &["worktree", "remove", &p], Some(dir))
                .with_context(|| format!("removing {p}"))?;
            act.run("git", &["branch", "-d", &w.branch], Some(dir))
                .with_context(|| format!("deleting {}", w.branch))?;
            writeln!(out, "removed {p} and {}", w.branch)?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    include!("prune_tests.rs");
}
