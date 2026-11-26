//! Without a rev: show where `dep` is pinned and fail when the repos disagree.
//! With a rev: rewrite each Cargo.toml that has `dep`, then refresh its lock.
use crate::scaffold::setup::Exec;
use actautoma::pin_manifest::{rev_of, with_rev};
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::PathBuf;

/// `folders` are the workspace repositories, as (name, directory).
pub fn run(
    folders: &[(String, PathBuf)],
    dep: &str,
    rev: Option<&str>,
    act: &impl Exec,
    out: &mut impl Write,
) -> Result<()> {
    let mut seen: Vec<(&str, String)> = vec![];
    for (name, dir) in folders {
        let file = dir.join("Cargo.toml");
        if !file.exists() {
            continue; // not a Rust repository: nothing to pin there
        }
        let text = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let Some(old) = rev_of(&text, dep).with_context(|| name.clone())? else {
            continue;
        };
        if let Some(rev) = rev {
            if old != rev {
                let new = with_rev(&text, dep, rev)?.unwrap_or(text); // present: rev_of found it
                std::fs::write(&file, new)
                    .with_context(|| format!("writing {}", file.display()))?;
                act.run("cargo", &["update", "-p", dep], Some(dir))
                    .with_context(|| format!("{name}: cargo update -p {dep} failed"))?;
            }
            writeln!(out, "{name:<25} {dep} -> {rev}")?;
        } else {
            writeln!(out, "{name:<25} {dep} {old}")?;
        }
        seen.push((name, old));
    }
    if seen.is_empty() {
        bail!("no workspace repository pins {dep} as a git dependency");
    }
    if rev.is_none() && seen.iter().any(|(_, r)| *r != seen[0].1) {
        bail!("{dep} is pinned to different revisions: run `actrepo pin {dep} <rev>`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("run_tests.rs");
}
