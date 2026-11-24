//! The dmg pipeline: Developer ID export, notarization, dmg, in that order.
use super::brew_beta::beta_version;
use super::brew_steps::{base_version, build_number};
use super::ctx::Ctx;
use super::macos_dmg;
use super::macos_export::export_app;
use super::macos_notarize::notarize;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

const TOOLS: [&str; 5] = ["flutter", "xcodebuild", "xcrun", "hdiutil", "ditto"];

/// Builds the signed, notarized beta dmg of the project and returns its path.
pub fn dmg(c: &mut Ctx) -> Result<PathBuf> {
    if let Some(tool) = TOOLS.iter().find(|t| !c.tool.on_path(t)) {
        bail!("{tool} is not on PATH (the dmg pipeline needs macOS with Xcode and Flutter).");
    }
    let base = base_version(c)?;
    let build = build_number(c)?;
    let app = export_app(c, &base, build)?;
    notarize(c, &app)?;
    let out_dir = c.root.join("build/macos/homebrew");
    let (path, sha) = macos_dmg::build(c, &app, &beta_version(&base, build), &out_dir)?;
    writeln!(c.out, "dmg {} sha256 {sha}", path.display()).context("reporting the dmg")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    include!("macos_pipeline_tests.rs");
}
