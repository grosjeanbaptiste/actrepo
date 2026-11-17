//! The real ports of `repo worktree`: git on the process, stdout.
use super::list::{json, list, text};
use super::owners::{assign, live};
use super::prune::prune;
use crate::scaffold::native::{Process, Quiet};
use anyhow::Result;
use std::path::Path;

pub fn run_prune(base: &str, dry_run: bool) -> Result<()> {
    let mut out = std::io::stdout().lock();
    let n = prune(&Quiet, &Process, Path::new("."), base, dry_run, &mut out)?;
    writeln!(
        out,
        "{n} worktree(s) {}",
        if dry_run { "to remove" } else { "removed" }
    )?;
    Ok(())
}

pub fn run_list(as_json: bool, with_owners: bool) -> Result<()> {
    let mut all = list(&Quiet, Path::new("."))?;
    if with_owners {
        assign(&mut all, &live(&Quiet)?);
    }
    if as_json {
        println!("{}", json(&all));
    } else {
        print!("{}", text(&all));
    }
    Ok(())
}

use std::io::Write;
