//! `repo <dir> run|install`: the real registry, the platform tools, the terminal.
use super::request::parse;
use crate::registry::file::{path, read};
use crate::run::resolve;
use actdevice::exec::System;
use actdevice::session::{discover, execute, names};
use anyhow::{Context, Result};

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
    execute(&System, &dir, req.action, &device, req.mode, &mut |w| {
        eprintln!("warn: {w}")
    })
    .map_err(|e| anyhow::anyhow!("{e}"))
    .with_context(|| format!("{} on {device}", req.name))
}
