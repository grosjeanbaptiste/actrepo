//! Remove each linked worktree whose branch was pushed and is now contained in
//! `origin/<base>`, then the branch; then each such local branch no worktree holds. Nothing is forced: a worktree with local
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

/// Linked worktrees on no branch whose HEAD is contained in `origin/<base>`: what a merge leaves behind
/// in the copy it ran from. They hold no work of their own.
fn detached_in_base(probe: &impl Exec, dir: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let list = probe.run("git", &["worktree", "list", "--porcelain"], Some(dir))?;
    let here = probe.run("git", &["rev-parse", "--show-toplevel"], Some(dir))?;
    let remote = format!("origin/{base}");
    let mut found = Vec::new();
    for block in list.split("\n\n").skip(1) {
        let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(key));
        if block.lines().any(|l| l.starts_with("prunable"))
            || !block.lines().any(|l| l == "detached")
        {
            continue;
        }
        let (Some(path), Some(head)) = (field("worktree "), field("HEAD ")) else {
            continue;
        };
        // an Err from merge-base --is-ancestor is its answer "not contained"
        let contained = probe
            .run(
                "git",
                &["merge-base", "--is-ancestor", head, &remote],
                Some(dir),
            )
            .is_ok();
        if contained && Path::new(path) != Path::new(here.trim()) {
            found.push(PathBuf::from(path));
        }
    }
    Ok(found)
}

/// Pushed once, contained in the base and not just the base itself; or adding nothing to the base.
fn merged(probe: &impl Exec, dir: &Path, branch: &str, base: &str) -> bool {
    let run = |args: &[&str]| probe.run("git", args, Some(dir));
    let (head, remote) = (format!("refs/heads/{branch}"), format!("origin/{base}"));
    // an Err from these is their answer (no upstream, not an ancestor), not a failure
    let pushed = run(&["config", "--get", &format!("branch.{branch}.remote")]).is_ok();
    let ancestor = run(&["merge-base", "--is-ancestor", &head, &remote]).is_ok();
    if ancestor {
        return pushed && run(&["rev-parse", &head]).ok() != run(&["rev-parse", &remote]).ok();
    }
    // With commits of its own, yet adding nothing to the base (squash-merged, reverted, or
    // only a merge of the base): `base...branch` is empty, so nothing is lost, pushed or not.
    run(&["diff", "--quiet", &format!("{remote}...{head}")]).is_ok()
}

/// Local branches that no worktree (the main one included) has checked out.
fn without_worktree(probe: &impl Exec, dir: &Path) -> Result<Vec<String>> {
    let list = probe.run("git", &["worktree", "list", "--porcelain"], Some(dir))?;
    let held: Vec<&str> = list
        .lines()
        .filter_map(|l| l.strip_prefix("branch refs/heads/"))
        .collect();
    let all = probe.run(
        "git",
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        Some(dir),
    )?;
    Ok(all
        .lines()
        .filter(|b| !held.contains(b))
        .map(str::to_string)
        .collect())
}

/// Deletes (or, with `dry_run`, names) the merged, pushed local branches no worktree holds: what a merge made elsewhere leaves behind. Returns how many.
fn delete_orphans(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    dry_run: bool,
    out: &mut impl Write,
) -> Result<usize> {
    let mut count = 0;
    for branch in without_worktree(probe, dir)? {
        if branch == base || !merged(probe, dir, &branch, base) {
            continue;
        }
        if dry_run {
            writeln!(out, "would delete branch {branch}")?;
        } else {
            // -D: containment in origin/BASE was just checked, as for a worktree's branch
            act.run("git", &["branch", "-D", &branch], Some(dir))
                .with_context(|| format!("deleting {branch}"))?;
            writeln!(out, "deleted branch {branch}")?;
        }
        count += 1;
    }
    Ok(count)
}

/// Returns how many worktrees and branches were (or, with `dry_run`, would be) removed.
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
            // -D: containment in origin/BASE was just checked; -d would compare with the local HEAD, which may be behind
            act.run("git", &["branch", "-D", &w.branch], Some(dir))
                .with_context(|| format!("deleting {}", w.branch))?;
            writeln!(out, "removed {p} and {}", w.branch)?;
            removed += 1;
        }
    }
    for path in detached_in_base(probe, dir, base)? {
        let p = path.display().to_string();
        if has_changes(probe, &path)? {
            writeln!(
                out,
                "kept {p}: contained in origin/{base} but the worktree has local changes"
            )?;
        } else if dry_run {
            writeln!(
                out,
                "would remove {p} (detached, contained in origin/{base})"
            )?;
            removed += 1;
        } else {
            act.run("git", &["worktree", "remove", &p], Some(dir))
                .with_context(|| format!("removing {p}"))?;
            writeln!(out, "removed {p} (detached, contained in origin/{base})")?;
            removed += 1;
        }
    }
    removed += delete_orphans(probe, act, dir, base, dry_run, out)?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    include!("prune_tests.rs");
}
