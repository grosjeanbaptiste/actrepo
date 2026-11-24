//! `actrepo worktree new BRANCH`: a linked worktree on a new branch cut from `origin/<base>`.
//! The branch tracks nothing: git would otherwise track `origin/<base>`, and a `git pull` in the
//! worktree would merge the base into the work.
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// The folder of the main worktree, found from `dir` even inside a linked worktree.
fn main_root(act: &impl Exec, dir: &Path) -> Result<PathBuf> {
    let common = act.run(
        "git",
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        Some(dir),
    )?;
    Path::new(common.trim())
        .parent()
        .map(Path::to_path_buf)
        .with_context(|| format!("no repository folder above {}", common.trim()))
}

pub fn create(act: &impl Exec, dir: &Path, branch: &str, base: &str) -> Result<PathBuf> {
    let path = main_root(act, dir)?
        .join(".worktrees")
        .join(branch.replace('/', "-"));
    let from = format!("origin/{base}");
    act.run(
        "git",
        &[
            "worktree",
            "add",
            "--no-track",
            "-b",
            branch,
            &path.display().to_string(),
            &from,
        ],
        Some(dir),
    )
    .with_context(|| format!("cannot create branch {branch} from {from}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    include!("new_tests.rs");
}
