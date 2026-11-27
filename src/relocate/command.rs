//! `actrepo move`, wired to the real machine: the folder, its worktrees, the registry row.
use super::plan::{Move, build, moved};
use super::worktrees::{list, repair};
use crate::registry::file::{Entry, path as registry_path, read};
use crate::registry::scan::write_atomically;
use anyhow::{Context, Result};
use std::path::Path;

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Move the folder, then repair what pointed at it. The registry row follows
/// the folder as soon as it has moved, so a failed repair never leaves it stale.
fn relocate(mv: &Move, entries: &[Entry], file: &Path) -> Result<()> {
    let worktrees = list(&mv.from)?;
    std::fs::rename(&mv.from, &mv.to).with_context(|| {
        format!(
            "cannot move {} to {} (a folder moves within one volume only)",
            mv.from.display(),
            mv.to.display()
        )
    })?;
    println!("moved {} -> {}", mv.from.display(), mv.to.display());
    if let Some(rows) = moved(entries, mv) {
        write_atomically(file, &rows).with_context(|| {
            format!("moved, but the registry {} was not updated", file.display())
        })?;
    }
    repair(&mv.to, &worktrees).context("moved, but the linked worktrees were not repaired")?;
    worktrees.iter().for_each(|w| println!("repaired worktree {}", w.display()));
    Ok(())
}

pub fn run(target: &str, into: &str, dry_run: bool) -> Result<()> {
    let file = registry_path(env)?;
    let entries = read(&file)?;
    let mv = build(&entries, target, Path::new(into), &|p| p.exists())?;
    if dry_run {
        println!("{} -> {}", mv.from.display(), mv.to.display());
        println!("dry run: nothing was changed");
        return Ok(());
    }
    relocate(&mv, &entries, &file)
}
