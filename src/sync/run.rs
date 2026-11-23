//! `actrepo sync [--dry-run] [DIR]`: fetch, read every branch of every repository, plan, then act.
use super::{
    exec::execute,
    plan::{Branch, plan, pull_only},
    read::{Facts, parse},
};
use crate::push::rows::collect;
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
            "--format=%(refname:short)%09%(upstream:short)%09%(upstream:track,nobracket)",
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

/// Prints the plan, then runs it unless `dry_run`. Fails when any step failed or needs a human.
pub fn run(
    dir: &Path,
    dry_run: bool,
    pull_only_mode: bool,
    exec: &impl Exec,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    let mut warnings = Vec::new();
    let rows = collect(exec, dir, &mut warnings)?;
    for w in warnings {
        writeln!(err, "warn: {w}")?;
    }
    let mut all = Vec::new();
    for row in &rows {
        all.extend(branches(exec, &row.repo, err).with_context(|| row.repo.display().to_string())?);
    }
    let steps = if pull_only_mode {
        pull_only(plan(&all))
    } else {
        plan(&all)
    };
    if dry_run {
        for s in &steps {
            writeln!(out, "plan {}", s.line())?;
        }
        if steps.is_empty() {
            writeln!(out, "nothing to sync")?;
        }
        return Ok(());
    }
    execute(&steps, exec, out)
}
