//! Homebrew beta release: GitHub prerelease carrying the dmg, `@beta` cask in the tap, bump, commit.
//! Needs a built dmg (`BREW_DMG`) and a checkout of the tap (`BREW_TAP_DIR`); pushes only when `BREW_PUSH` is set.
use super::brew::bump;
use super::brew_beta::{asset_name, beta_cask, beta_version, tag};
use super::ctx::Ctx;
use super::stores;
use super::tool::{Call, plain};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

fn path_var(c: &Ctx, name: &str) -> Result<PathBuf> {
    c.vars
        .var(name)
        .map(PathBuf::from)
        .with_context(|| format!("set {name}."))
}

/// `BREW_BUILD`, else the number of commits of the project: each beta is newer than the last.
fn build_number(c: &Ctx) -> Result<u64> {
    if let Some(b) = c.vars.var("BREW_BUILD") {
        return b
            .parse()
            .with_context(|| format!("BREW_BUILD is not a number: {b}."));
    }
    let args = ["rev-list", "--count", "HEAD"].map(String::from);
    let call = Call {
        cwd: Some(c.root),
        ..plain("git", &args)
    };
    let out = c.tool.capture(&call)?.and_then(|n| n.parse().ok());
    out.context("cannot count the commits (set BREW_BUILD).")
}

/// The beta cask of the tap, made from the stable one when it does not exist yet.
fn ensure_beta_cask(casks: &Path, name: &str) -> Result<()> {
    let beta = casks.join(format!("{name}@beta.rb"));
    if beta.exists() {
        return Ok(());
    }
    let stable = casks.join(format!("{name}.rb"));
    if !stable.exists() {
        bail!(
            "no stable cask {} to make the beta cask from.",
            stable.display()
        );
    }
    fs::write(&beta, beta_cask(&fs::read_to_string(&stable)?)?)
        .with_context(|| format!("writing {}", beta.display()))
}

/// The dmg copied under the name the cask url expects, in a folder of its own.
fn stage(dmg: &Path, asset: &str, tag: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("actrepo-brew-{tag}"));
    fs::create_dir_all(&dir)?;
    let staged = dir.join(asset);
    fs::copy(dmg, &staged).with_context(|| format!("copying {}", dmg.display()))?;
    Ok(staged)
}

fn git(c: &Ctx, tap: &Path, args: &[&str]) -> Result<()> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    c.tool
        .run(&Call {
            cwd: Some(tap),
            ..plain("git", &args)
        })
        .with_context(|| format!("git {} in the tap", args[0]))
}

/// Publishes the beta: prerelease, `@beta` cask bumped to it, commit in the tap (and push on request).
pub fn publish(c: &mut Ctx) -> Result<()> {
    let dmg = path_var(c, "BREW_DMG")?;
    if !dmg.is_file() {
        bail!(
            "BREW_DMG not found: {} (build it with build_dmg.sh).",
            dmg.display()
        );
    }
    let tap = path_var(c, "BREW_TAP_DIR")?;
    let app = c.vars.var("BREW_APP").unwrap_or_else(|| "Acteble".into());
    let repo = c
        .vars
        .var("BREW_REPO")
        .unwrap_or_else(|| "acteble/homebrew-acteble".into());
    let base = match c.vars.var("BREW_VERSION") {
        Some(v) => v,
        None => stores::pubspec_version(c.root)?
            .context("cannot resolve version (set BREW_VERSION).")?,
    };
    let version = beta_version(&base, build_number(c)?);
    let (tag, asset) = (tag(&version), asset_name(&app, &version));
    ensure_beta_cask(&tap.join("Casks"), &app.to_lowercase())?;
    let staged = stage(&dmg, &asset, &tag)?;
    let release = [
        "release",
        "create",
        &tag,
        "--repo",
        &repo,
        "--prerelease",
        "--title",
        &tag,
        "--notes",
        "Beta build",
        &staged.display().to_string(),
    ]
    .map(String::from);
    c.tool
        .run(&plain("gh", &release))
        .context("creating the GitHub prerelease")?;
    let url = format!("https://github.com/{repo}/releases/download/{tag}/{asset}");
    bump(c, &version, &url, &tap, "beta")?;
    git(c, &tap, &["add", "Casks"])?;
    git(
        c,
        &tap,
        &["commit", "-m", &format!("chore: {app} beta {version}")],
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
