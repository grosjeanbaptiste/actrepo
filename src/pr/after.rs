//! After `gh pr merge`: bring the local branches up to date. The PR is merged
//! by then, so every failing step is reported and the call fails, but none
//! hides that the merge itself succeeded.
use crate::scaffold::setup::Exec;
use crate::worktree::prune::prune;
use anyhow::{Result, bail};
use std::io::Write;
use std::path::{Path, PathBuf};

/// `dir` is a checkout of the repository; `probe` reads quietly, `act` changes things.
pub fn update(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    head: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    writeln!(out, "\nUpdating local branches...")?;
    let git = |args: &[&str]| act.run("git", args, Some(dir));
    if git(&["fetch", "-p"]).is_err() {
        writeln!(err, "warning: PR merged; git fetch failed.")?;
        bail!("git fetch failed after the merge");
    }
    let Ok(holder) = holder(probe, dir, base) else {
        writeln!(
            err,
            "warning: PR merged; the worktrees could not be listed, {base} was not updated."
        )?;
        bail!("worktrees could not be listed after the merge");
    };
    let mut ok = match holder {
        None => advance_here(&git, base, err)?,
        Some(path) => advance_there(probe, act, dir, &path, base, err)?,
    };
    if merge_verified(probe, dir, base, head, err)? {
        ok &= release_head(probe, act, dir, head, err)?;
        ok &= delete_head(probe, &git, dir, head, err)?;
        ok &= delete_remote_head(probe, &git, dir, head, err)?;
        ok &= sweep_merged(probe, act, dir, base, out, err)?;
    } else {
        ok = false;
    }
    if ok {
        Ok(())
    } else {
        bail!("PR merged, but the local update is incomplete")
    }
}

/// Another worktree (not `dir`'s own) that has `branch` checked out.
fn holder(probe: &impl Exec, dir: &Path, branch: &str) -> Result<Option<PathBuf>> {
    let here = probe.run("git", &["rev-parse", "--show-toplevel"], Some(dir))?;
    let list = probe.run("git", &["worktree", "list", "--porcelain"], Some(dir))?;
    let (wanted, mut path) = (format!("branch refs/heads/{branch}"), "");
    for line in list.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p;
        } else if line == wanted && Path::new(path) != Path::new(here.trim()) {
            return Ok(Some(PathBuf::from(path)));
        }
    }
    Ok(None)
}

fn advance_here(
    git: &dyn Fn(&[&str]) -> Result<String>,
    base: &str,
    err: &mut impl Write,
) -> Result<bool> {
    if git(&["checkout", base])
        .and_then(|_| git(&["pull"]))
        .is_ok()
    {
        return Ok(true);
    }
    writeln!(err, "warning: PR merged; {base} was not updated.")?;
    Ok(false)
}

fn advance_there(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    holder: &Path,
    base: &str,
    err: &mut impl Write,
) -> Result<bool> {
    let h = holder.display().to_string();
    let remote = format!("origin/{base}");
    let mut ok = true;
    if has_changes(probe, holder)? {
        writeln!(
            err,
            "warning: PR merged; {base} is not updated: its worktree {h} has local changes."
        )?;
        ok = false;
    } else if act
        .run("git", &["-C", &h, "merge", "--ff-only", &remote], None)
        .is_err()
    {
        writeln!(
            err,
            "warning: PR merged; {base} could not be fast-forwarded in {h}."
        )?;
        ok = false;
    }
    Ok(ok
        & act
            .run("git", &["checkout", "--detach", &remote], Some(dir))
            .is_ok())
}

fn ref_exists(probe: &impl Exec, dir: &Path, refname: &str) -> bool {
    // an Err from show-ref is its answer "no such ref", not a failure
    let args = ["show-ref", "--verify", "--quiet", refname];
    probe.run("git", &args, Some(dir)).is_ok()
}

/// The merge is checked before anything is removed: the head's tip (local, else
/// remote) must be contained in `origin/<base>`. A head that exists nowhere has
/// nothing to protect.
fn merge_verified(
    probe: &impl Exec,
    dir: &Path,
    base: &str,
    head: &str,
    err: &mut impl Write,
) -> Result<bool> {
    let local = format!("refs/heads/{head}");
    let remote = format!("refs/remotes/origin/{head}");
    let Some(tip) = [local, remote]
        .into_iter()
        .find(|r| ref_exists(probe, dir, r))
    else {
        return Ok(true);
    };
    let base_ref = format!("origin/{base}");
    // an Err from merge-base --is-ancestor is its answer "not contained"
    let contained = ["merge-base", "--is-ancestor", &tip, &base_ref];
    if probe.run("git", &contained, Some(dir)).is_ok() {
        return Ok(true);
    }
    writeln!(
        err,
        "warning: PR merged; {head} is not contained in {base}, so its worktree and branches are kept."
    )?;
    Ok(false)
}

/// `gh pr merge --delete-branch` normally removed it; one left behind goes now that the merge is verified.
fn delete_remote_head(
    probe: &impl Exec,
    git: &dyn Fn(&[&str]) -> Result<String>,
    dir: &Path,
    head: &str,
    err: &mut impl Write,
) -> Result<bool> {
    if !ref_exists(probe, dir, &format!("refs/remotes/origin/{head}")) {
        return Ok(true);
    }
    if git(&["push", "origin", "--delete", head]).is_ok() {
        return Ok(true);
    }
    writeln!(
        err,
        "warning: PR merged; remote branch {head} was not deleted."
    )?;
    Ok(false)
}

/// Other worktrees whose branch got merged earlier (by any tool) go too.
fn sweep_merged(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<bool> {
    match prune(probe, act, dir, base, false, out) {
        Ok(_) => Ok(true),
        Err(e) => {
            writeln!(
                err,
                "warning: PR merged; merged worktrees were not pruned: {e:#}"
            )?;
            Ok(false)
        }
    }
}

/// Uncommitted or untracked files in the worktree at `path`.
pub(crate) fn has_changes(probe: &impl Exec, path: &Path) -> Result<bool> {
    let p = path.display().to_string();
    let status = ["--no-optional-locks", "-C", &p, "status", "--porcelain"];
    Ok(!probe.run("git", &status, None)?.trim().is_empty())
}

/// A worktree that still holds the merged head branch is removed, so the branch
/// can go. One with local changes is kept and reported: nothing is forced.
fn release_head(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    head: &str,
    err: &mut impl Write,
) -> Result<bool> {
    let Some(path) = holder(probe, dir, head)? else {
        return Ok(true);
    };
    let p = path.display();
    if has_changes(probe, &path)? {
        writeln!(
            err,
            "warning: PR merged; worktree {p} has local changes, so it and {head} are kept."
        )?;
        return Ok(false);
    }
    if act
        .run("git", &["worktree", "remove", &p.to_string()], Some(dir))
        .is_err()
    {
        writeln!(
            err,
            "warning: PR merged; worktree {p} could not be removed."
        )?;
        return Ok(false);
    }
    Ok(true)
}

/// `gh pr merge --delete-branch` may have deleted it already: only a branch that
/// is still there and refuses deletion is a failure.
fn delete_head(
    probe: &impl Exec,
    git: &dyn Fn(&[&str]) -> Result<String>,
    dir: &Path,
    head: &str,
    err: &mut impl Write,
) -> Result<bool> {
    let refname = format!("refs/heads/{head}");
    // an Err from show-ref is its answer "no such branch", not a failure
    if probe
        .run(
            "git",
            &["show-ref", "--verify", "--quiet", &refname],
            Some(dir),
        )
        .is_err()
    {
        return Ok(true);
    }
    if git(&["branch", "-d", head]).is_ok() {
        return Ok(true);
    }
    writeln!(
        err,
        "warning: PR merged; local branch {head} was not deleted."
    )?;
    Ok(false)
}

#[cfg(test)]
mod tests {
    include!("after_tests.rs");
}
