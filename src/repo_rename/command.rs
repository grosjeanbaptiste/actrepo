//! `repo rename NEW [--dir DIR] [--local-only] [--dry-run]`.
use super::plan::{Env, Opts, plan};
use crate::registry::file::state_root;
use actautoma::forge::{Forge, Runner};
use actautoma::gh::Gh;
use actautoma::process::ProcessRunner;
use actautoma::rename::{check, execute};
use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn run(
    opts: &Opts,
    env: &Env,
    forge: &impl Forge,
    runner: &impl Runner,
    out: &mut impl Write,
    err: &mut impl Write,
) -> Result<()> {
    let origin = runner
        .run("git", &["remote", "get-url", "origin"], Some(&opts.dir))
        .with_context(|| format!("{} has no origin remote to rename", opts.dir.display()))?;
    let plan = plan(opts, env, &origin, &|p| p.is_dir(), err)?;
    if opts.dry_run {
        check(&plan)?;
        for r in plan.renames() {
            writeln!(out, "{} -> {}", r.from, r.to)?;
        }
        writeln!(out, "dry run: nothing was changed")?;
        return Ok(());
    }
    let done = execute(&plan, forge, runner)?;
    writeln!(
        out,
        "moved {} -> {}",
        plan.old_dir.display(),
        plan.new_dir().display()
    )?;
    for (root, report) in &done.reports {
        let (c, n, l) = (
            report.changed.len(),
            report.not_text.len(),
            report.links.len(),
        );
        writeln!(
            out,
            "{}: {c} rewritten, {n} not text, {l} links",
            root.display()
        )?;
    }
    if let Some(bin) = &done.binary {
        writeln!(out, "renamed binary {}", bin.display())?;
    }
    for path in &done.moved {
        writeln!(out, "renamed {}", path.display())?;
    }
    writeln!(out, "run `repo scan` to refresh the registry names")?;
    Ok(())
}

fn env_from(var: impl Fn(&str) -> Option<String>) -> Result<Env> {
    let home = PathBuf::from(var("HOME").context("HOME is not set")?);
    let cargo = var("CARGO_HOME").map_or_else(|| home.join(".cargo"), PathBuf::from);
    let dotfiles = ["PROJECT_REPO_ROOT", "DOTFILES_REPO_ROOT"]
        .iter()
        .find_map(|k| var(k).filter(|v| !v.is_empty()))
        .map_or_else(|| home.join("dotfiles"), PathBuf::from);
    Ok(Env {
        state: state_root(&var)?,
        dotfiles,
        cargo_bin: cargo.join("bin"),
        home,
    })
}

pub fn run_rename(new: &str, dir: Option<&str>, local_only: bool, dry_run: bool) -> Result<()> {
    let env = env_from(|k| std::env::var(k).ok())?;
    let dir = std::path::absolute(Path::new(dir.unwrap_or(".")))?;
    let opts = Opts {
        new: new.to_string(),
        dir,
        local_only,
        dry_run,
    };
    let (mut out, mut err) = (std::io::stdout().lock(), std::io::stderr().lock());
    run(
        &opts,
        &env,
        &Gh::new(ProcessRunner),
        &ProcessRunner,
        &mut out,
        &mut err,
    )
}

#[cfg(test)]
mod tests {
    include!("command_tests.rs");
}
