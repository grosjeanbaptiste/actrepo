//! The `post-commit hook` line of `actrepo check|apply`, so a project that has the standard
//! files also has the hook that pushes after each commit.
use super::install::{Installed, install};
use crate::scaffold::apply::Mode;
use crate::scaffold::setup::Exec;
use std::path::Path;

/// Installs the hook (`Apply`, `Force`) or only reports it (`Check`, `Diff`), as one report line.
pub fn line(exec: &impl Exec, dir: &Path, mode: Mode) -> String {
    let write = matches!(mode, Mode::Apply | Mode::Force);
    match install(exec, dir, !write) {
        Ok(Installed::Unchanged(_)) => "  = ok       post-commit hook".to_string(),
        Ok(Installed::Written(_)) if write => "  + wrote      post-commit hook".to_string(),
        Ok(Installed::Written(_)) if mode == Mode::Diff => {
            "  - would write  post-commit hook".to_string()
        }
        Ok(Installed::Written(_)) => "  - missing  post-commit hook".to_string(),
        Err(e) if format!("{e}").contains("another post-commit hook") => {
            format!("  ! kept     post-commit hook: {e}")
        }
        Err(e) => format!("  - skipped  post-commit hook: {e:#}"),
    }
}

#[cfg(test)]
#[path = "line_tests.rs"]
mod tests;
