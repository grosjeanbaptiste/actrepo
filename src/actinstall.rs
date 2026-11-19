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
    match act.run("actinstall", &["update", "--repo", &main], None) {
        Ok(_) => Ok(true),
        Err(e) => {
            writeln!(err, "warning: actinstall update failed for {main}: {e:#}")?;
            Ok(false)
        }
    }
}
