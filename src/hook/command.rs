//! The real ports of `actrepo hook`: git on the process, the current folder, the log file.
use super::detach::{detach, log_path};
use super::install::{Installed, install_all};
use super::secrets::{Mode, scan};
use super::sync::{Outcome, sync};
use crate::scaffold::native::Process;
use anyhow::{Context, Result};
use std::path::Path;

pub fn run_install(dir: &Path, dry_run: bool) -> Result<()> {
    for installed in install_all(&Process, dir, dry_run)? {
        match installed {
            Installed::Written(f) if dry_run => println!("would write {}", f.display()),
            Installed::Written(f) => println!("wrote {}", f.display()),
            Installed::Unchanged(f) => println!("unchanged {}", f.display()),
        }
    }
    Ok(())
}

/// Called by the `pre-commit` (staged) and `pre-push` stubs: an error blocks the git action.
pub fn run_secrets(staged: bool) -> Result<()> {
    let mode = if staged { Mode::Staged } else { Mode::Unpushed };
    scan(&Process, Path::new("."), mode)
}

/// Called by the `post-commit` stub: starts the sync in the background.
pub fn run_detached() -> Result<()> {
    let log = log_path(
        std::env::var("XDG_STATE_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )?;
    let exe = std::env::current_exe().context("finding the actrepo binary")?;
    detach(&exe, Path::new("."), &log)
}

/// Pulls and pushes the repository of the current folder; says what it did.
pub fn run_sync() -> Result<()> {
    match sync(&Process, Path::new("."))? {
        Outcome::Skipped(why) => println!("skipped: {why:?}"),
        Outcome::Synced { pulled } => println!("pushed (pulled first: {pulled})"),
    }
    Ok(())
}
