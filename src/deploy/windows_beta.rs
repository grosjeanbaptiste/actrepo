//! Windows beta: the `.msix` on the beta prerelease, where `install.ps1 -Beta` reads it.
//! Needs the package (`BREW_WINDOWS_MSIX`). The stable Windows channel is the Microsoft Store (`msstore`).
use super::brew_beta::tag;
use super::brew_channel::Channel;
use super::brew_steps::{attach_to_beta, base_version, build_number, path_var, stage};
use super::ctx::Ctx;
use super::target::Env;
use actautoma::forge::NewRelease;
use anyhow::{Result, bail};

pub fn publish(c: &mut Ctx) -> Result<()> {
    if c.env == Env::Prod {
        bail!("the Windows GitHub release is a beta: deploy it to test.");
    }
    let msix = path_var(c, "BREW_WINDOWS_MSIX")?;
    if !msix.is_file() {
        bail!("BREW_WINDOWS_MSIX not found: {}.", msix.display());
    }
    let app = c.vars.var("BREW_APP").unwrap_or_else(|| "Acteble".into());
    let repo = c
        .vars
        .var("BREW_REPO")
        .unwrap_or_else(|| "acteble/homebrew-acteble".into());
    let version = Channel::Beta.version(&base_version(c)?, || build_number(c))?;
    let tag = tag(&version);
    let staged = stage(&msix, &format!("{app}-{version}.msix"), &tag)?;
    let release = NewRelease {
        slug: repo.clone(),
        tag: tag.clone(),
        title: Channel::Beta.title(&app, &version, &tag),
        notes: Channel::Beta.notes().into(),
        prerelease: true,
        asset: Some(staged),
    };
    attach_to_beta(c, &repo, &release)
}

#[cfg(test)]
mod tests {
    include!("windows_beta_tests.rs");
}
