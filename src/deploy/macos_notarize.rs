//! Slice 2 of the dmg pipeline: notarize and staple the `.app` (port of `notarize_macos_app.sh`).
use super::ctx::Ctx;
use super::tool::plain;
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

fn run(c: &Ctx, what: &str, prog: &str, args: &[&str]) -> Result<()> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    c.tool
        .run(&plain(prog, &args))
        .with_context(|| format!("{what} failed"))
}

fn submit_and_staple(
    c: &Ctx,
    app: &str,
    zip: &str,
    key: &str,
    id: &str,
    issuer: &str,
) -> Result<()> {
    run(c, "ditto", "ditto", &["-c", "-k", "--keepParent", app, zip])?;
    let submit = [
        "notarytool",
        "submit",
        zip,
        "--key",
        key,
        "--key-id",
        id,
        "--issuer",
        issuer,
        "--wait",
    ];
    run(c, "xcrun notarytool submit", "xcrun", &submit)?;
    run(
        c,
        "xcrun stapler staple",
        "xcrun",
        &["stapler", "staple", app],
    )?;
    run(
        c,
        "spctl --assess",
        "spctl",
        &["--assess", "--type", "execute", "-vv", app],
    )?;
    run(
        c,
        "xcrun stapler validate",
        "xcrun",
        &["stapler", "validate", app],
    )
}

/// Notarizes `app` with the App Store Connect API key and staples the ticket to it.
pub fn notarize(c: &mut Ctx, app: &Path) -> Result<()> {
    let id = c.vars.var("ASC_KEY_ID").context("set ASC_KEY_ID.")?;
    let issuer = c.vars.var("ASC_ISSUER_ID").context("set ASC_ISSUER_ID.")?;
    let key = c
        .vars
        .path_or("ASC_PRIVATE_KEY_PATH", ".config/asc/AuthKey.p8");
    if !key.is_file() {
        bail!("ASC_PRIVATE_KEY_PATH not found: {}.", key.display());
    }
    if !app.exists() {
        bail!("no .app at {} to notarize.", app.display());
    }
    let zip = app.with_extension("zip");
    let (app_s, zip_s, key_s) = (
        app.display().to_string(),
        zip.display().to_string(),
        key.display().to_string(),
    );
    let result = submit_and_staple(c, &app_s, &zip_s, &key_s, &id, &issuer);
    match fs::remove_file(&zip) {
        Err(e) if e.kind() != ErrorKind::NotFound => {
            writeln!(c.err, "warning: could not remove {zip_s}: {e}")
                .context("reporting the leftover zip")?;
        }
        _ => {} // the zip is gone, or the chain failed before making it
    }
    result
}

#[cfg(test)]
mod tests {
    include!("macos_notarize_tests.rs");
}
