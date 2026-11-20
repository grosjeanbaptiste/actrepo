//! The real ports of `actrepo pin`: the workspace file, cargo on the process, stdout.
use crate::scaffold::native::Quiet;
use crate::workspace::{command::workspace_file, file::folders};
use anyhow::{Context, Result};
use std::path::Path;

pub fn command(dep: &str, rev: Option<&str>) -> Result<()> {
    let ws = workspace_file()?;
    let json = std::fs::read_to_string(&ws).with_context(|| format!("reading {}", ws.display()))?;
    let base = ws.parent().unwrap_or(Path::new("."));
    let found: Vec<_> = folders(&json)?
        .into_iter()
        .map(|f| (f.name, base.join(f.path)))
        .collect();
    super::run::run(&found, dep, rev, &Quiet, &mut std::io::stdout().lock())
}
