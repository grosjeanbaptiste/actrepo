//! Slice 1 of the dmg pipeline: the macOS `.app` signed with a Developer ID identity (port of `build_macos_homebrew.sh`).
use super::ctx::Ctx;
use super::target::Env;
use super::tool::{Call, plain};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

const ARCHIVE: &str = "build/macos/archive-homebrew/Runner.xcarchive";
const EXPORT: &str = "build/macos/homebrew/export";

fn step(c: &Ctx, what: &str, prog: &str, args: &[String]) -> Result<()> {
    let call = Call {
        cwd: Some(c.root),
        ..plain(prog, args)
    };
    c.tool.run(&call).with_context(|| format!("{what} failed"))
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

fn required(c: &Ctx, rel: &str) -> Result<String> {
    let path = c.root.join(rel);
    if !path.is_file() {
        bail!(
            "missing {rel} under {} (Developer ID signing).",
            c.root.display()
        );
    }
    Ok(path.display().to_string())
}

fn clear(dir: &Path) -> Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(e).with_context(|| format!("clearing {}", dir.display()))
        }
        _ => Ok(()), // nothing left over from an earlier export
    }
}

fn the_app(dir: &Path) -> Result<PathBuf> {
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

/// Builds, archives with Developer ID and exports the `.app`; returns its path.
pub fn export_app(c: &Ctx, version: &str, build: u64) -> Result<PathBuf> {
    let team = c.vars.var("APPLE_TEAM_ID").context("set APPLE_TEAM_ID.")?;
    let identity = c
        .vars
        .var("MACOS_SIGN_IDENTITY")
        .unwrap_or_else(|| "Developer ID Application".into());
    let options = required(c, "macos/ExportOptionsDeveloperID.plist")?;
    let entitlements = required(c, "macos/Runner/DeveloperID.entitlements")?;
    clear(&c.root.join(ARCHIVE))?;
    clear(&c.root.join(EXPORT))?;
    let build = build.to_string();
    let flutter = [
        "build",
        "macos",
        "--release",
        "--build-name",
        version,
        "--build-number",
        &build,
    ];
    step(c, "flutter build macos", "flutter", &strings(&flutter))?;
    let mut archive = strings(&[
        "-workspace",
        "macos/Runner.xcworkspace",
        "-scheme",
        "Runner",
        "-configuration",
        "Release",
        "-allowProvisioningUpdates",
        "archive",
        "-archivePath",
        ARCHIVE,
    ]);
    archive.extend([
        "CODE_SIGN_STYLE=Manual".into(),
        format!("CODE_SIGN_IDENTITY={identity}"),
        format!("DEVELOPMENT_TEAM={team}"),
        format!("CODE_SIGN_ENTITLEMENTS={entitlements}"),
        "ENABLE_HARDENED_RUNTIME=YES".into(),
        "OTHER_CODE_SIGN_FLAGS=--options=runtime".into(),
    ]);
    step(c, "xcodebuild archive", "xcodebuild", &archive)?;
    let export = [
        "-exportArchive",
        "-archivePath",
        ARCHIVE,
        "-exportPath",
        EXPORT,
        "-exportOptionsPlist",
        &options,
        "-allowProvisioningUpdates",
    ];
    step(
        c,
        "xcodebuild -exportArchive",
        "xcodebuild",
        &strings(&export),
    )?;
    the_app(&c.root.join(EXPORT))
}

#[cfg(test)]
mod tests {
    include!("macos_export_tests.rs");
}
