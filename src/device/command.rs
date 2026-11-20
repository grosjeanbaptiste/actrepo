//! `repo <dir> run|install`: the real registry, the platform tools, the terminal.
use super::request::parse;
use crate::kind::detect;
use crate::registry::file::{path, read};
use crate::run::resolve;
use actdevice::exec::System;
use actdevice::session::{discover, execute, names};
use anyhow::{Context, Result, bail};

pub fn command(args: &[String]) -> Result<()> {
    let req = parse(args)?;
    let entries = read(&path(|k| std::env::var(k).ok())?)?;
    let dir = resolve(&req.name, &entries)?;
    let Some(device) = req.device else {
        let found = discover(&System)?;
        found.warnings.iter().for_each(|w| eprintln!("warn: {w}"));
        println!("{}", names(&found).join("\n"));
        return Ok(());
    };
    let kind = detect(&dir)?;
    let Some(tooling) = kind.device_tooling() else {
        bail!(
            "{} is a {kind:?} project: it does not run on a device",
            req.name
        );
    };
    execute(
        &System,
        tooling,
        &dir,
        req.action,
        &device,
        req.mode,
        &mut |w| eprintln!("warn: {w}"),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
    .with_context(|| format!("{} on {device}", req.name))
}

/// `actrepo __complete WORDS…`: one candidate per line.
pub fn complete(words: &[String]) -> Result<()> {
    let entries = read(&path(|k| std::env::var(k).ok())?)?;
    let devices = || -> Result<Vec<String>> {
        let found = discover(&System)?;
        found.warnings.iter().for_each(|w| eprintln!("warn: {w}"));
        Ok(found
            .devices
            .iter()
            .map(|d| format!("{}\t{}", d.id, d.name))
            .collect())
    };
    for line in super::complete::candidates(words, &entries, &devices)? {
        println!("{line}");
    }
    Ok(())
}
