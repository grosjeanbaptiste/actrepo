//! `actrepo docs-paths FILE... [--write]`: check, or fix, the Path column against the registry.
use super::{Entry, fix};
use anyhow::{Result, bail};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn run(
    files: &[PathBuf],
    write: bool,
    entries: &[Entry],
    home: &Path,
    out: &mut dyn Write,
) -> Result<()> {
    let mut stale = 0;
    for file in files {
        let text = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", file.display()))?;
        let fixed = fix(&text, entries, home);
        for row in &fixed.changed {
            writeln!(out, "{}: {row}", file.display())?;
        }
        for name in &fixed.unknown {
            writeln!(out, "{}: not in the registry: {name}", file.display())?;
        }
        stale += fixed.changed.len();
        if write && !fixed.changed.is_empty() {
            std::fs::write(file, &fixed.text)
                .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", file.display()))?;
        }
    }
    if stale > 0 && !write {
        bail!("{stale} stale path(s); run again with --write to fix them");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("command_tests.rs");
}
