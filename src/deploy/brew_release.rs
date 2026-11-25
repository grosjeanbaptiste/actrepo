//! Homebrew release: GitHub (pre)release carrying the dmg, the channel cask of the tap bumped to it, commit.
//! prod publishes the stable release, test the `@beta` prerelease.
//! Needs a built dmg (`BREW_DMG`) and a checkout of the tap (`BREW_TAP_DIR`); pushes only when `BREW_PUSH` is set.
use super::brew::bump;
use super::brew_beta::{asset_name, tag};
use super::brew_channel::Channel;
use super::brew_steps::{base_version, build_number, ensure_beta_cask, git, path_var, stage};
use super::ctx::Ctx;
use super::macos_pipeline;
use actautoma::forge::NewRelease;
use anyhow::{Context, Result, bail};
use std::path::Path;

/// Publishes the release of the channel: release, cask bumped to it, commit in the tap (and push on request).
pub fn publish(c: &mut Ctx) -> Result<()> {
    let dmg = path_var(c, "BREW_DMG")?;
    publish_dmg(c, &dmg)
}

/// Builds the dmg (export, notarize, dmg), then publishes it; the tap is checked first, before the long build.
pub fn build_and_publish(c: &mut Ctx) -> Result<()> {
    path_var(c, "BREW_TAP_DIR")?;
    let dmg = macos_pipeline::dmg(c)?;
    publish_dmg(c, &dmg)
}

fn publish_dmg(c: &mut Ctx, dmg: &Path) -> Result<()> {
    if !dmg.is_file() {
        bail!(
            "BREW_DMG not found: {} (unset BREW_DMG to build it).",
            dmg.display()
        );
    }
    let tap = path_var(c, "BREW_TAP_DIR")?;
    let app = c.vars.var("BREW_APP").unwrap_or_else(|| "Acteble".into());
    let repo = c
        .vars
        .var("BREW_REPO")
        .unwrap_or_else(|| "acteble/homebrew-acteble".into());
    let base = base_version(c)?;
    let channel = Channel::of(c.env);
    let version = channel.version(&base, || build_number(c))?;
    let (tag, asset) = (tag(&version), asset_name(&app, &version));
    if channel == Channel::Beta {
        ensure_beta_cask(&tap.join("Casks"), &app.to_lowercase())?;
    }
    let staged = stage(dmg, &asset, &tag)?;
    let release = NewRelease {
        slug: repo.clone(),
        tag: tag.clone(),
        title: channel.title(&app, &version, &tag),
        notes: channel.notes().into(),
        prerelease: channel.prerelease(),
        asset: Some(staged),
    };
    c.forge.create_release(&release).context(format!(
        "creating the GitHub {}",
        if channel.prerelease() {
            "prerelease"
        } else {
            "release"
        }
    ))?;
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    bump(c, &version, &url, &tap, channel.name())?;
    git(c, &tap, &["add", "Casks"])?;
    git(
        c,
        &tap,
        &["commit", "-m", &channel.commit_message(&app, &version)],
    )?;
    if c.vars.var("BREW_PUSH").is_some() {
        git(c, &tap, &["push"])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("brew_release_tests.rs");
}
