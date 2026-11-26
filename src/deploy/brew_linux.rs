//! Homebrew beta for Linux: the prebuilt bundle on the beta prerelease, the tap's `<app>-linux@beta` formula bumped to it.
//! Needs the tarball (`BREW_LINUX_TARBALL`) and a checkout of the tap (`BREW_TAP_DIR`); pushes only when `BREW_PUSH` is set.
//! Stable stays `scripts/release_homebrew_linux.sh`: a public release needs a human at the keyboard.
use super::brew_beta::tag;
use super::brew_beta_formula::bump_formula;
use super::brew_channel::Channel;
use super::brew_steps::{
    attach_to_beta, base_version, build_number, ensure_beta_formula, git, path_var, stage,
};
use super::ctx::Ctx;
use super::macos_dmg::sha256;
use super::target::Env;
use actautoma::forge::NewRelease;
use anyhow::{Context, Result, bail};
use std::fs;

pub fn publish(c: &mut Ctx) -> Result<()> {
    if c.env == Env::Prod {
        bail!(
            "the Linux Homebrew release is a beta: deploy it to test (stable: scripts/release_homebrew_linux.sh)."
        );
    }
    let tarball = path_var(c, "BREW_LINUX_TARBALL")?;
    if !tarball.is_file() {
        bail!("BREW_LINUX_TARBALL not found: {}.", tarball.display());
    }
    let tap = path_var(c, "BREW_TAP_DIR")?;
    let app = c.vars.var("BREW_APP").unwrap_or_else(|| "Acteble".into());
    let repo = c
        .vars
        .var("BREW_REPO")
        .unwrap_or_else(|| "acteble/homebrew-acteble".into());
    let name = format!("{}-linux", app.to_lowercase());
    let asset = format!("{name}-x86_64.tar.gz");
    ensure_beta_formula(&tap.join("Formula"), &name)?;
    let version = Channel::Beta.version(&base_version(c)?, || build_number(c))?;
    let tag = tag(&version);
    let sum = sha256(c, &tarball)?;
    let staged = stage(&tarball, &asset, &tag)?;
    let release = NewRelease {
        slug: repo.clone(),
        tag: tag.clone(),
        title: Channel::Beta.title(&app, &version, &tag),
        notes: Channel::Beta.notes().into(),
        prerelease: true,
        asset: Some(staged),
    };
    attach_to_beta(c, &repo, &release)?;
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    let file = tap.join("Formula").join(format!("{name}@beta.rb"));
    let bumped = bump_formula(&fs::read_to_string(&file)?, &version, &url, &sum)?;
    fs::write(&file, bumped).with_context(|| format!("writing {}", file.display()))?;
    git(c, &tap, &["add", "Formula"])?;
    let message = format!("chore: {app} linux beta {version}");
    git(c, &tap, &["commit", "-m", &message])?;
    if c.vars.var("BREW_PUSH").is_some() {
        git(c, &tap, &["push"])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("brew_linux_tests.rs");
}
