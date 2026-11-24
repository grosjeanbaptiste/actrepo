//! `actrepo sync [--dry-run] [DIR]`: fetch, read every branch of every repository, plan, then act.
use super::{
    exec::{execute, execute_noting},
    plan::{Branch, plan, pull_only},
    read::{Facts, parse},
};
use crate::push::rows::{Row, collect, enclosing};
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

fn git(exec: &impl Exec, repo: &Path, args: &[&str]) -> Result<String> {
    exec.run("git", args, Some(repo))
}

/// The branches of one repository, after a fetch so the counts are current.
fn branches(exec: &impl Exec, repo: &Path, err: &mut impl Write) -> Result<Vec<Branch>> {
    if let Err(e) = git(exec, repo, &["fetch", "--prune", "--quiet", "origin"]) {
        writeln!(
            err,
            "warn: {}: fetch failed, counts may be stale: {e:#}",
            repo.display()
        )?;
    }
    let refs = git(
        exec,
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)%09%(upstream:short)%09%(upstream:track,nobracket)%09%(worktreepath)",
            "refs/heads",
        ],
    )?;
    let current =
        git(exec, repo, &["symbolic-ref", "--quiet", "--short", "HEAD"]).unwrap_or_default(); // detached HEAD: nothing is checked out
    let default = git(
        exec,
        repo,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .unwrap_or_default(); // no origin/HEAD: default branch unknown
    let dirty = !git(exec, repo, &["status", "--porcelain=v1"])?
        .trim()
        .is_empty();
    let facts = Facts {
        refs: &refs,
        current: current.trim(),
        dirty,
        default_branch: default.trim().strip_prefix("origin/").unwrap_or(""),
    };
    parse(repo, &facts)
}

/// What `actrepo sync` was asked: which steps to keep and how far to look.
#[derive(Clone, Copy, Default)]
pub struct Opts {
    pub dry_run: bool,
    /// Keep only the pulls (nothing is pushed).
    pub pull_only: bool,
    /// Only the repository containing the folder; nothing below it is scanned.
    pub here: bool,
}

/// The repositories to settle: everything under `dir`, or only the enclosing one.
fn repositories(
    exec: &impl Exec,
    dir: &Path,
    here: bool,
    warnings: &mut Vec<String>,
) -> Result<Vec<Row>> {
    if here {
        return Ok(enclosing(exec, dir)?.into_iter().collect());
    }
    collect(exec, dir, warnings)
}

/// Prints the plan, then runs it unless `dry_run`. Fails when any step failed or needs a human.
pub fn run(
    dir: &Path,
    opts: Opts,
    exec: &impl Exec,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    let mut warnings = Vec::new();
    let rows = repositories(exec, dir, opts.here, &mut warnings)?;
    if rows.is_empty() {
        writeln!(out, "not in a git repository: nothing to sync")?;
        return Ok(());
    }
    for w in warnings {
        writeln!(err, "warn: {w}")?;
    }
    let mut all = Vec::new();
    for row in &rows {
        all.extend(branches(exec, &row.repo, err).with_context(|| row.repo.display().to_string())?);
    }
    let steps = if opts.pull_only {
        pull_only(plan(&all))
    } else {
        plan(&all)
    };
    if opts.dry_run {
        for s in &steps {
            writeln!(out, "plan {}", s.line())?;
        }
        if steps.is_empty() {
            writeln!(out, "nothing to sync")?;
        }
        return Ok(());
    }
    if opts.pull_only {
        return execute_noting(&steps, exec, out);
    }
    execute(&steps, exec, out)
}
