//! `actrepo skills link [--dry-run]`, wired to the registry and `~/.claude/skills`.
use super::plan::{Step, apply, only, plan};
use crate::registry::file;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn run_link(names: &[String], dry_run: bool) -> Result<()> {
    let entries = file::read(&file::path(|k| std::env::var(k).ok())?)?;
    let home = std::env::var("HOME").context("HOME is not set")?;
    let steps = plan(&entries, &PathBuf::from(home).join(".claude/skills"))?;
    let steps = only(steps, names)?;
    for s in &steps {
        match s {
            Step::Link { name, link, .. } => println!("link     {name} -> {}", link.display()),
            Step::Linked { name } => println!("linked   {name}"),
            Step::Conflict { name, reason } => println!("conflict {name}: {reason}"),
        }
    }
    if dry_run {
        return Ok(());
    }
    apply(&steps)
}
