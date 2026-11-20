//! `actrepo rename`, wired to the real machine and, through actautoma, the hosting platform.
use super::entry::renamed;
use super::plan::{Machine, Request, build};
use super::slug::qualify;
use crate::registry::file::{Entry, read, state_root};
use crate::registry::scan::write_atomically;
use actautoma::rename::{Outcome, Plan, check, execute};
use actautoma::{gh::Gh, process::ProcessRunner};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// The folders the old name may be written in, from the environment.
fn machine() -> Result<Machine> {
    let home = PathBuf::from(env("HOME").context("HOME is not set")?);
    let cargo = env("CARGO_HOME").map_or_else(|| home.join(".cargo"), PathBuf::from);
    Ok(Machine {
        state: state_root(env)?,
        dotfiles: env("DOTFILES_REPO_ROOT")
            .or_else(|| env("PROJECT_REPO_ROOT"))
            .map_or_else(|| home.join("dotfiles"), PathBuf::from),
        bin_dir: cargo.join("bin"),
        home,
    })
}

fn report(plan: &Plan, out: &Outcome) {
    println!(
        "moved {} -> {}",
        plan.old_dir.display(),
        plan.new_dir().display()
    );
    for (root, r) in &out.reports {
        println!(
            "{}: {} rewritten, {} not text, {} links",
            root.display(),
            r.changed.len(),
            r.not_text.len(),
            r.links.len()
        );
    }
    if let Some(bin) = &out.binary {
        println!(
            "renamed binary {} (run `cargo install --path` to refresh cargo's record)",
            bin.display()
        );
    }
    out.moved
        .iter()
        .for_each(|p| println!("renamed {}", p.display()));
}

/// `origin` of the git checkout in `dir`.
fn origin_url(dir: &Path) -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["remote", "get-url", "origin"])
        .output()
        .context("cannot start git")?;
    anyhow::ensure!(
        out.status.success(),
        "git remote get-url origin failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn run(
    slug: &str,
    new: &str,
    dir: Option<&str>,
    into: Option<&str>,
    local_only: bool,
    dry_run: bool,
) -> Result<()> {
    let machine = machine()?;
    let file = machine.state.join("projects.txt");
    let entries = read(&file)?;
    let mut known = entries.clone();
    if let Some(dir) = dir {
        known.push(Entry {
            name: slug.into(),
            path: PathBuf::from(dir),
        }); // --dir names the folder of a bare OLD too
    }
    let slug = qualify(slug, &known, &|dir| origin_url(dir))?;
    let req = Request {
        slug: &slug,
        new,
        dir: dir.map(Path::new),
        local_only,
        into: into.map(Path::new),
    };
    let planned = build(&req, &entries, &machine, &|p| p.is_dir())?;
    planned.warnings.iter().for_each(|w| eprintln!("warn: {w}"));
    let plan = planned.plan;
    if dry_run {
        check(&plan)?;
        plan.renames()
            .iter()
            .for_each(|r| println!("{} -> {}", r.from, r.to));
        println!("dry run: nothing was changed");
        return Ok(());
    }
    let out = execute(&plan, &Gh::new(ProcessRunner), &ProcessRunner)?;
    report(&plan, &out);
    match renamed(&entries, &plan.old, &plan.new, &plan.new_dir()) {
        Some(rows) => write_atomically(&file, &rows).with_context(|| {
            format!(
                "renamed everywhere, but the registry {} was not updated",
                file.display()
            )
        })?,
        None => eprintln!(
            "warn: {} was not in the registry {}",
            plan.old,
            file.display()
        ),
    }
    Ok(())
}
