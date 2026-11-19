//! The copy a PR was merged from: a linked worktree on the PR's branch. Once the merge is verified it
//! has nothing left to hold, so it goes too — the main worktree and any other worktree never do.
use super::after::has_changes;
use crate::actinstall::main_worktree;
use crate::scaffold::setup::Exec;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

fn git(probe: &impl Exec, dir: &Path, args: &[&str]) -> Result<String> {
    probe
        .run("git", args, Some(dir))
        .map(|s| s.trim().to_string())
}

/// Checked before the update moves anything: `dir` is a linked worktree currently on `head`.
pub fn is_own_copy(probe: &impl Exec, dir: &Path, head: &str) -> Result<bool> {
    let on = git(probe, dir, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default(); // detached HEAD: on nothing
    if on != head {
        return Ok(false);
    }
    let own = git(probe, dir, &["rev-parse", "--absolute-git-dir"])?;
    let common = git(
        probe,
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    Ok(own != common)
}

/// Removes the copy (never forced) from the main worktree; one with local changes, or one that is not at
/// `origin/<base>` after the update, is kept with a `warning:`. `Ok(false)` when it is kept.
pub fn remove(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<bool> {
    // `dir` may be `.`: the main worktree needs the real path
    let top = git(probe, dir, &["rev-parse", "--show-toplevel"])?;
    let here = &top;
    if has_changes(probe, dir)? {
        writeln!(
            err,
            "warning: PR merged; worktree {here} has local changes, so it is kept."
        )?;
        return Ok(false);
    }
    let (head, tip) = (
        git(probe, dir, &["rev-parse", "HEAD"])?,
        git(probe, dir, &["rev-parse", &format!("origin/{base}")])?,
    );
    if head != tip {
        writeln!(
            err,
            "warning: PR merged; worktree {here} is not at origin/{base}, so it is kept."
        )?;
        return Ok(false);
    }
    let main = main_worktree(probe, dir)?;
    if act
        .run("git", &["worktree", "remove", &top], Some(Path::new(&main)))
        .is_err()
    {
        writeln!(
            err,
            "warning: PR merged; worktree {here} could not be removed."
        )?;
        return Ok(false);
    }
    writeln!(out, "Removed this worktree ({here}). Continue in {main}.")?;
    Ok(true)
}
