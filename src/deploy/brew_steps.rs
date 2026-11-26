//! Steps of the Homebrew beta release: variables, build number, beta cask, staged dmg, git in the tap.
use super::brew_beta::beta_cask;
use super::brew_beta_formula::{beta_file_name, beta_formula};
use super::ctx::Ctx;
use super::tool::{Call, plain};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub fn path_var(c: &Ctx, name: &str) -> Result<PathBuf> {
    c.vars
        .var(name)
        .map(PathBuf::from)
        .with_context(|| format!("set {name}."))
}

/// `BREW_BUILD`, else the number of commits of the project: each beta is newer than the last.
pub fn build_number(c: &Ctx) -> Result<u64> {
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
pub fn ensure_beta_cask(casks: &Path, name: &str) -> Result<()> {
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

/// The beta formula of the tap (Linux), made from the stable one when it does not exist yet.
pub fn ensure_beta_formula(formulas: &Path, name: &str) -> Result<()> {
    let beta = formulas.join(beta_file_name(name));
    if beta.exists() {
        return Ok(());
    }
    let stable = formulas.join(format!("{name}.rb"));
    if !stable.exists() {
        bail!(
            "no stable formula {} to make the beta formula from.",
            stable.display()
        );
    }
    fs::write(&beta, beta_formula(&fs::read_to_string(&stable)?)?)
        .with_context(|| format!("writing {}", beta.display()))
}

/// The dmg copied under the name the cask url expects, in a folder of its own.
pub fn stage(dmg: &Path, asset: &str, tag: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("actrepo-brew-{tag}"));
    fs::create_dir_all(&dir)?;
    let staged = dir.join(asset);
    fs::copy(dmg, &staged).with_context(|| format!("copying {}", dmg.display()))?;
    Ok(staged)
}

pub fn git(c: &Ctx, tap: &Path, args: &[&str]) -> Result<()> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    c.tool
        .run(&Call {
            cwd: Some(tap),
            ..plain("git", &args)
        })
        .with_context(|| format!("git {} in the tap", args[0]))
}

/// `BREW_VERSION`, else the pubspec version of the project.
pub fn base_version(c: &Ctx) -> Result<String> {
    match c.vars.var("BREW_VERSION") {
        Some(v) => Ok(v),
        None => super::stores::pubspec_version(c.root)?
            .context("cannot resolve version (set BREW_VERSION)."),
    }
}

#[cfg(test)]
mod tests {
    include!("brew_steps_formula_tests.rs");
}
