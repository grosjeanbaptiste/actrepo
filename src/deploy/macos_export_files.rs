//! The files around the Developer ID export: the signing inputs it needs and the `.app` it leaves.
use super::ctx::Ctx;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// A signing input of the project, as an absolute path; named in the error when it is missing.
pub fn required(c: &Ctx, rel: &str) -> Result<String> {
    let path = c.root.join(rel);
    if !path.is_file() {
        bail!(
            "missing {rel} under {} (Developer ID signing).",
            c.root.display()
        );
    }
    Ok(path.display().to_string())
}

/// Removes `dir` with what it holds; a folder that is not there is already clear.
pub fn clear(dir: &Path) -> Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(e).with_context(|| format!("clearing {}", dir.display()))
        }
        _ => Ok(()), // nothing left over from an earlier export
    }
}

/// The one `.app` the export left in `dir`.
pub fn the_app(dir: &Path) -> Result<PathBuf> {
    let found: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "app"))
            .collect(),
        Err(_) => vec![], // no export folder: the export produced nothing
    };
    match found.as_slice() {
        [one] => Ok(one.clone()),
        [] => bail!("the export produced no .app under {}.", dir.display()),
        many => bail!(
            "the export produced {} .app under {}, expected one.",
            many.len(),
            dir.display()
        ),
    }
}
