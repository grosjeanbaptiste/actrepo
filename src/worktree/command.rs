//! The real ports of `repo worktree`: git on the process, stdout.
use super::list::{json, list, text};
use super::prune::prune;
use crate::actinstall::refresh;
use crate::scaffold::native::{Process, Quiet};
use crate::scaffold::setup::Exec;
use anyhow::Result;
use std::path::Path;

pub fn run_prune(base: &str, dry_run: bool) -> Result<()> {
    let (mut out, mut err) = (std::io::stdout().lock(), std::io::stderr().lock());
    prune_and_refresh(
        &Quiet,
        &Process,
        Path::new("."),
        base,
        dry_run,
        &mut out,
        &mut err,
    )
}

/// A pruned worktree means its branch was merged: the installed binaries of the repository are refreshed.
pub fn prune_and_refresh(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    base: &str,
    dry_run: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    let n = prune(probe, act, dir, base, dry_run, out)?;
    writeln!(
        out,
        "{n} worktree(s) {}",
        if dry_run { "to remove" } else { "removed" }
    )?;
    if n > 0 && !dry_run && !refresh(probe, act, dir, err)? {
        anyhow::bail!("worktrees pruned, but actinstall did not update the installed binaries");
    }
    Ok(())
}

pub fn run_list(as_json: bool) -> Result<()> {
    let all = list(&Quiet, Path::new("."))?;
    if as_json {
        println!("{}", json(&all));
    } else {
        print!("{}", text(&all));
    }
    Ok(())
}

use std::io::Write;
