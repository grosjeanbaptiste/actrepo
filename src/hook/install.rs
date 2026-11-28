//! `actrepo hook install`: put the `post-commit` file in a repository's hooks folder.
use super::secrets::{PRE_COMMIT, PRE_PUSH};
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

/// Writes one hook file; refuses to replace a different hook of the same name.
fn put(file: PathBuf, name: &str, text: &str, dry_run: bool) -> Result<Installed> {
    match fs::read_to_string(&file) {
        Ok(current) if current == text => return Ok(Installed::Unchanged(file)),
        Ok(_) => bail!(
            "{} already holds another {name} hook: not overwritten",
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
    fs::write(&file, text).with_context(|| format!("writing {}", file.display()))?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("making {} executable", file.display()))?;
    Ok(Installed::Written(file))
}

/// Writes the `post-commit` hook; refuses to replace a different one.
pub fn install(exec: &impl Exec, dir: &Path, dry_run: bool) -> Result<Installed> {
    put(
        hooks_dir(exec, dir)?.join("post-commit"),
        "post-commit",
        HOOK,
        dry_run,
    )
}

/// Writes `post-commit` (sync) and `pre-commit` / `pre-push` (secret scan). Every hook is
/// tried; a foreign or unwritable one never stops the others, and all failures come back
/// together in one error.
pub fn install_all(exec: &impl Exec, dir: &Path, dry_run: bool) -> Result<Vec<Installed>> {
    let hooks = hooks_dir(exec, dir)?;
    let wanted = [
        ("post-commit", HOOK),
        ("pre-commit", PRE_COMMIT),
        ("pre-push", PRE_PUSH),
    ];
    let (mut done, mut failed) = (Vec::new(), Vec::new());
    for (name, text) in wanted {
        match put(hooks.join(name), name, text, dry_run) {
            Ok(i) => done.push(i),
            Err(e) => failed.push(format!("{e:#}")),
        }
    }
    if failed.is_empty() {
        Ok(done)
    } else {
        bail!("{}", failed.join("\n"))
    }
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "install_all_tests.rs"]
mod all_tests;
