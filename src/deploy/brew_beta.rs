//! Homebrew beta channel, pure part: version, tag, asset name and the `@beta` cask made from the stable one.
//! The beta is a GitHub prerelease `v<version>-beta.<build>`; its cask conflicts with the stable cask.
use anyhow::{Result, bail};

/// `1.2.3` (a pubspec `+build` suffix is dropped) and a build number: `1.2.3-beta.42`.
pub fn beta_version(version: &str, build: u64) -> String {
    let base = version.split('+').next().unwrap_or(version);
    format!("{base}-beta.{build}")
}

pub fn tag(version: &str) -> String {
    format!("v{version}")
}

/// The cask url is `<App>-<version>.dmg`, so the beta asset carries the beta version.
pub fn asset_name(app: &str, version: &str) -> String {
    format!("{app}-{version}.dmg")
}

/// The `<name>` of a `cask "<name>" do` line.
fn cask_name(line: &str) -> Option<&str> {
    line.strip_prefix("cask \"")?.strip_suffix("\" do")
}

/// The `@beta` cask of a stable cask: renamed, in conflict with the stable one, `desc` marked.
pub fn beta_cask(stable: &str) -> Result<String> {
    let Some(name) = stable.lines().find_map(cask_name) else {
        bail!("not a stable cask: no `cask \"<name>\" do` line.");
    };
    if name.ends_with("@beta") {
        bail!("already a beta cask: {name}.");
    }
    let mut out = String::new();
    for line in stable.lines() {
        out.push_str(&beta_line(line, name));
        out.push('\n');
    }
    Ok(out)
}

fn beta_line(line: &str, name: &str) -> String {
    if cask_name(line).is_some() {
        return format!("cask \"{name}@beta\" do\n  conflicts_with cask: \"{name}\"");
    }
    match line.trim_start().strip_prefix("desc \"") {
        Some(d) if d.ends_with('"') => format!("  desc \"{} (beta)\"", &d[..d.len() - 1]),
        _ => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    include!("brew_beta_tests.rs");
}
