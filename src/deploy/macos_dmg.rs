//! Slice 3 of the dmg pipeline: the notarized `.app` and an `/Applications` link in a `.dmg` (port of `build_dmg.sh`).
use super::brew_beta::asset_name;
use super::ctx::Ctx;
use super::tool::plain;
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGES: AtomicU64 = AtomicU64::new(0);

fn run(c: &Ctx, what: &str, prog: &str, args: &[String]) -> Result<()> {
    c.tool
        .run(&plain(prog, args))
        .with_context(|| format!("{what} failed"))
}

fn remove(path: &Path) -> Result<()> {
    let gone = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    match gone {
        Err(e) if e.kind() != ErrorKind::NotFound => {
            Err(e).with_context(|| format!("removing {}", path.display()))
        }
        _ => Ok(()), // nothing left over from an earlier run
    }
}

/// `ditto` keeps the signature and the symlinks of the app; the `Applications` link makes it drag-installable.
pub fn stage_app(c: &Ctx, app: &Path, stage: &Path) -> Result<()> {
    let name = app.file_name().context("the .app has no name")?;
    let copy = stage.join(name);
    run(
        c,
        "ditto",
        "ditto",
        &[app.display().to_string(), copy.display().to_string()],
    )?;
    std::os::unix::fs::symlink("/Applications", stage.join("Applications"))
        .context("linking /Applications into the stage")
}

fn sha256(c: &Ctx, dmg: &Path) -> Result<String> {
    let args = ["-a", "256", &dmg.display().to_string()].map(String::from);
    let out = c
        .tool
        .capture(&plain("shasum", &args))?
        .context("could not hash the dmg (shasum failed).")?;
    out.split_whitespace()
        .next()
        .map(String::from)
        .context("could not hash the dmg (empty shasum output).")
}

fn create(c: &Ctx, app: &Path, name: &str, dmg: &Path, stage: &Path) -> Result<()> {
    stage_app(c, app, stage)?;
    let args = [
        "create",
        "-volname",
        name,
        "-srcfolder",
        &stage.display().to_string(),
        "-ov",
        "-format",
        "UDZO",
        &dmg.display().to_string(),
    ]
    .map(String::from);
    run(c, "hdiutil create", "hdiutil", &args)
}

/// Packs `app` into `<out_dir>/<App>-<version>.dmg` and returns it with its sha256.
pub fn build(c: &mut Ctx, app: &Path, version: &str, out_dir: &Path) -> Result<(PathBuf, String)> {
    if !app.exists() {
        bail!("no .app at {} to pack.", app.display());
    }
    let name = c.vars.var("BREW_APP").unwrap_or_else(|| "Acteble".into());
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let dmg = out_dir.join(asset_name(&name, version));
    remove(&dmg)?;
    let n = STAGES.fetch_add(1, Ordering::Relaxed); // one folder per call: two builds in a process never share a stage
    let stage = std::env::temp_dir().join(format!("actrepo-dmg-{}-{n}", std::process::id()));
    remove(&stage)?;
    fs::create_dir_all(&stage)?;
    let made = create(c, app, &name, &dmg, &stage);
    let cleaned = remove(&stage);
    made.and(cleaned)?;
    if !dmg.is_file() {
        bail!("hdiutil produced no dmg at {}.", dmg.display());
    }
    Ok((dmg.clone(), sha256(c, &dmg)?))
}

#[cfg(test)]
mod tests {
    include!("macos_dmg_tests.rs");
}
