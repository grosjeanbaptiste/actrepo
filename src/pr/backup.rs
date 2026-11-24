//! A copy of the pull request's branch, taken before the merge: `refs/backup/<head>/<tip>`.
use crate::scaffold::setup::Exec;
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::Path;

/// Fetch the remote tip of `head` and pin it under `refs/backup/<head>/<tip>`.
/// The name carries the tip, so an earlier backup of a reused branch name is never replaced.
/// Any failure is returned: the caller must not merge without the copy.
pub fn save(
    probe: &impl Exec,
    act: &impl Exec,
    dir: &Path,
    head: &str,
    out: &mut impl Write,
) -> Result<String> {
    let fetch = format!("refs/heads/{head}");
    act.run("git", &["fetch", "origin", &fetch], Some(dir))
        .with_context(|| format!("backup of {head}: git fetch origin {fetch} failed"))?;
    let tip = probe
        .run("git", &["rev-parse", "--verify", "FETCH_HEAD"], Some(dir))
        .with_context(|| format!("backup of {head}: reading the fetched tip failed"))?;
    let tip = tip.trim();
    if tip.is_empty() {
        bail!("backup of {head}: git rev-parse --verify FETCH_HEAD returned no commit");
    }
    let name = format!("refs/backup/{head}/{tip}");
    act.run("git", &["update-ref", &name, tip], Some(dir))
        .with_context(|| format!("backup of {head}: git update-ref {name} failed"))?;
    writeln!(out, "Backup: {name}")?;
    Ok(name)
}
