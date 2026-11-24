//! `actrepo hook install`: put the `post-commit` file in a repository's hooks folder.
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Git runs hooks as files, so this one is a shell stub that only calls the Rust
/// binary (the written exception to the Rust-only rule: git accepts nothing else).
pub const HOOK: &str = "#!/bin/sh\n# Installed by `actrepo hook install`: git runs hooks as files, this stub only calls the Rust binary.\nexec actrepo hook run\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Installed {
    Written(PathBuf),
    Unchanged(PathBuf),
}

/// Where git looks for hooks in `dir` (honours `core.hooksPath` and linked worktrees).
fn hooks_dir(exec: &impl Exec, dir: &Path) -> Result<PathBuf> {
    let out = exec
        .run("git", &["rev-parse", "--git-path", "hooks"], Some(dir))
        .context("not a git repository")?;
    Ok(dir.join(out.trim()))
}

/// Writes the hook; refuses to replace a different `post-commit` hook.
pub fn install(exec: &impl Exec, dir: &Path, dry_run: bool) -> Result<Installed> {
    let file = hooks_dir(exec, dir)?.join("post-commit");
    match fs::read_to_string(&file) {
        Ok(text) if text == HOOK => return Ok(Installed::Unchanged(file)),
        Ok(_) => bail!(
            "{} already holds another post-commit hook: not overwritten",
            file.display()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).with_context(|| format!("reading {}", file.display())),
    }
    if dry_run {
        return Ok(Installed::Written(file));
    }
    let parent = file.parent().context("hook path has no folder")?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::write(&file, HOOK).with_context(|| format!("writing {}", file.display()))?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("making {} executable", file.display()))?;
    Ok(Installed::Written(file))
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
