//! Snap Store (Linux): upload a built `.snap`; prod -> `stable`, test -> `beta`.
use super::ctx::Ctx;
use super::find;
use super::target::Env;
use super::tool::{Call, plain};
use anyhow::{Context, Result, bail};

pub fn upload(c: &mut Ctx) -> Result<()> {
    if !c.tool.on_path("snapcraft") {
        bail!("snapcraft not found — Snap publish needs Linux + `snapcraft login`.");
    }
    let credentials = match c.vars.var("SNAPCRAFT_STORE_CREDENTIALS") {
        Some(v) => Some(v),
        None => stored(c)?,
    };
    let Some(credentials) = credentials else {
        bail!(
            "no Snap Store credentials — enroll with:\n  snapcraft export-login - | actesskey add snapcraft-store-credentials"
        );
    };
    let build = c.root.join("build");
    let Some(snap) = find::deep(&build, "snap") else {
        bail!(
            "no .snap under {} (run `snapcraft pack` first).",
            build.display()
        );
    };
    let channel = if c.env == Env::Prod { "stable" } else { "beta" };
    let args = [
        "upload".to_string(),
        snap.display().to_string(),
        format!("--release={channel}"),
    ];
    let env = [("SNAPCRAFT_STORE_CREDENTIALS".to_string(), credentials)];
    c.tool
        .run(&Call {
            env: &env,
            ..plain("snapcraft", &args)
        })
        .context("uploading the snap")
}

/// The enrolled credentials: `actesskey get` first, then `dotfiles secret get` while the
/// secrets are still migrating. An empty answer counts as none.
fn stored(c: &mut Ctx) -> Result<Option<String>> {
    if c.tool.on_path("actesskey") {
        let args = ["get", "snapcraft-store-credentials"].map(String::from);
        let found = c.tool.capture(&plain("actesskey", &args))?;
        if let Some(v) = found.filter(|v| !v.is_empty()) {
            return Ok(Some(v));
        }
    }
    let args = ["secret", "get", "snapcraft-store-credentials"].map(String::from);
    Ok(c.tool
        .capture(&plain("dotfiles", &args))?
        .filter(|v| !v.is_empty()))
}

#[cfg(test)]
mod tests {
    include!("snap_tests.rs");
}
