//! Code that landed on a default branch changes what is installed: `actinstall` (its own repository)
//! brings the installed binaries of this repository up to date. `repo` only asks, and reports its failure.
use crate::scaffold::setup::Exec;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

/// The main worktree of the repository `dir` belongs to: the first block of git's listing.
pub(crate) fn main_worktree(probe: &impl Exec, dir: &Path) -> Result<String> {
    let list = probe.run("git", &["worktree", "list", "--porcelain"], Some(dir))?;
    let first = list.split("\n\n").next().unwrap_or_default();
    first
        .lines()
        .find_map(|l| l.strip_prefix("worktree "))
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("git worktree list: no main worktree in {list:?}"))
}

/// Seconds `repo` waits for `actinstall update` (it queues behind other heavy jobs).
pub fn wait_from_env(env: impl Fn(&str) -> Option<String>) -> Result<u64> {
    match env(WAIT_VAR) {
        None => Ok(DEFAULT_WAIT),
        Some(v) => v.parse::<u64>().ok().filter(|s| *s > 0).ok_or_else(|| {
            anyhow::anyhow!("{WAIT_VAR} must be a whole number of seconds above 0, got {v:?}")
        }),
    }
}

const WAIT_VAR: &str = "REPO_ACTINSTALL_WAIT";
const DEFAULT_WAIT: u64 = 600;

/// Runs `actinstall update --repo <main worktree>`; `Ok(false)` after a `warning:` when it cannot.
pub fn refresh(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    err: &mut impl Write,
) -> Result<bool> {
    let main = match main_worktree(probe, dir) {
        Ok(main) => main,
        Err(e) => {
            writeln!(err, "warning: actinstall was not run: {e:#}")?;
            return Ok(false);
        }
    };
    let secs = wait_from_env(|k| std::env::var(k).ok())?;
    match act.run_within("actinstall", &["update", "--repo", &main], None, secs) {
        Ok(_) => Ok(true),
        Err(e) => {
            writeln!(err, "warning: actinstall update failed for {main}: {e:#}")?;
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    include!("actinstall_tests.rs");
}
