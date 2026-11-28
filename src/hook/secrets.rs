//! `actrepo hook secrets`: refuse a commit or a push that carries a secret, through gitleaks.
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use std::path::Path;

/// Git runs hooks as files, so these are shell stubs that only call the Rust binary
/// (the written exception to the Rust-only rule: git accepts nothing else).
pub const PRE_COMMIT: &str = "#!/bin/sh\n# Installed by `actrepo hook install`: git runs hooks as files, this stub only calls the Rust binary.\nexec actrepo hook secrets --staged\n";
pub const PRE_PUSH: &str = "#!/bin/sh\n# Installed by `actrepo hook install`: git runs hooks as files, this stub only calls the Rust binary.\nexec actrepo hook secrets\n";

/// What to read: the staged changes (commit) or the commits no remote holds yet (push).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Staged,
    Unpushed,
}

/// Runs gitleaks on `dir`. A leak, a missing gitleaks or any other failure is an error:
/// there is no fallback that lets the commit or the push through.
pub fn scan(exec: &impl Exec, dir: &Path, mode: Mode) -> Result<()> {
    let mut args = vec!["git"];
    match mode {
        Mode::Staged => args.push("--staged"),
        Mode::Unpushed => args.extend(["--log-opts", "HEAD --not --remotes"]),
    }
    args.extend(["--redact", "--no-banner", "--exit-code", "1", "."]);
    exec.run("gitleaks", &args, Some(dir))
        .map(drop)
        .with_context(|| "secret scan (gitleaks) refused this: fix the finding, or install gitleaks (`dotfiles add gitleaks`)")
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod tests;
