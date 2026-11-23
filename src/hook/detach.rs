//! `actrepo hook run`: what the `post-commit` stub calls. It starts `actrepo hook sync`
//! in the background, its output appended to the log, and returns at once so the
//! commit never waits for the push gate.
use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// `$XDG_STATE_HOME/actrepo/hook.log`, else `~/.local/state/actrepo/hook.log`.
pub fn log_path(xdg: Option<&str>, home: Option<&str>) -> Result<PathBuf> {
    match (
        xdg.filter(|s| !s.is_empty()),
        home.filter(|s| !s.is_empty()),
    ) {
        (Some(x), _) => Ok(Path::new(x).join("actrepo/hook.log")),
        (None, Some(h)) => Ok(Path::new(h).join(".local/state/actrepo/hook.log")),
        (None, None) => {
            anyhow::bail!("neither XDG_STATE_HOME nor HOME is set: no place for the hook log")
        }
    }
}

/// Starts `exe hook sync` in `dir` with its output appended to `log`; does not wait.
pub fn detach(exe: &Path, dir: &Path, log: &Path) -> Result<()> {
    let parent = log.parent().context("log path has no folder")?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .with_context(|| format!("opening {}", log.display()))?;
    let err = file.try_clone().context("duplicating the log handle")?;
    Command::new(exe)
        .args(["hook", "sync"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(err)
        .spawn()
        .with_context(|| format!("starting {} hook sync", exe.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "detach_tests.rs"]
mod tests;
