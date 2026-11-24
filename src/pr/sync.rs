//! Bring a pull request's branch up to date with its base so a PR that GitHub
//! reports CONFLICTING only because the base moved can still be merged.
//! Works in a throwaway worktree: the caller's checkout is never touched.
use crate::scaffold::setup::Exec;
use anyhow::{Result, bail};
use std::io::Write;
use std::path::{Path, PathBuf};

fn scratch(head: &str) -> PathBuf {
    let safe: String = head
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    std::env::temp_dir().join(format!("actrepo-sync-{}-{safe}", std::process::id()))
}

/// Merge `origin/<base>` into `head` and push it. A merge that fails is a real
/// content conflict: it is aborted, nothing is pushed, and the error says so.
pub fn update_branch(
    act: &impl Exec,
    dir: &Path,
    base: &str,
    head: &str,
    err: &mut impl Write,
) -> Result<()> {
    if act.run("git", &["fetch", "origin"], Some(dir)).is_err() {
        writeln!(err, "could not fetch origin to update {head}.")?;
        bail!("git fetch origin failed");
    }
    let tmp = scratch(head);
    let tmp_s = tmp.to_string_lossy().into_owned();
    let remote_head = format!("origin/{head}");
    if let Err(e) = act.run(
        "git",
        &["worktree", "add", "--detach", &tmp_s, &remote_head],
        Some(dir),
    ) {
        writeln!(err, "could not check out {head} to update it: {e}")?;
        bail!("git worktree add failed");
    }
    let result = merge_and_push(act, &tmp, base, head, err);
    if let Err(e) = act.run("git", &["worktree", "remove", "--force", &tmp_s], Some(dir)) {
        writeln!(err, "warning: could not remove {tmp_s}: {e}")?;
        if result.is_ok() {
            bail!("git worktree remove failed");
        }
    }
    result
}

fn merge_and_push(
    act: &impl Exec,
    tmp: &Path,
    base: &str,
    head: &str,
    err: &mut impl Write,
) -> Result<()> {
    let git = |args: &[&str]| act.run("git", args, Some(tmp));
    save_original(act, tmp, head, err)?; // no merge into the branch without the copy
    if git(&["merge", "--no-edit", &format!("origin/{base}")]).is_err() {
        if let Err(e) = git(&["merge", "--abort"]) {
            writeln!(err, "warning: git merge --abort failed: {e}")?;
        }
        writeln!(err, "{head} conflicts with {base}: resolve it by hand.")?;
        bail!("{head} conflicts with {base}");
    }
    if git(&["push", "origin", &format!("HEAD:{head}")]).is_err() {
        writeln!(err, "could not push the updated {head}.")?;
        bail!("git push failed");
    }
    Ok(())
}

/// Push the current tip of `head` (the checkout in `tmp`) to `refs/backup/<head>/<tip>`.
fn save_original(act: &impl Exec, tmp: &Path, head: &str, err: &mut impl Write) -> Result<()> {
    let tip = act.run("git", &["rev-parse", "--verify", "HEAD"], Some(tmp))?;
    let tip = tip.trim();
    if tip.is_empty() {
        writeln!(err, "could not read the tip of {head} to copy it.")?;
        bail!("git rev-parse --verify HEAD returned no commit");
    }
    let to = format!("HEAD:refs/backup/{head}/{tip}");
    if act.run("git", &["push", "origin", &to], Some(tmp)).is_err() {
        writeln!(err, "could not push the backup of {head}.")?;
        bail!("git push of the backup of {head} failed");
    }
    Ok(())
}
