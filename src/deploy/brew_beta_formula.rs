//! Homebrew beta channel for Linux, pure part: the `@beta` formula made from the stable one.
//! Casks are macOS-only, so Linux ships as a formula; its beta is `<name>@beta`, in conflict with the stable one.
use anyhow::{Result, bail};

/// `<Name>` of a `class <Name> < Formula` line.
fn class_name(line: &str) -> Option<&str> {
    line.strip_prefix("class ")?.strip_suffix(" < Formula")
}

/// The file of the beta formula of `name` (`acteble-linux` gives `acteble-linux@beta.rb`).
pub fn beta_file_name(name: &str) -> String {
    format!("{name}@beta.rb")
}

/// The Homebrew name of a formula class, back to the formula: `ActebleLinux` is `acteble-linux`.
fn formula_name(class: &str) -> String {
    let mut name = String::new();
    for (i, c) in class.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                name.push('-');
            }
            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }
    name
}

/// The `@beta` formula of a stable formula: class renamed as Homebrew does for `<name>@beta`
/// (`ActebleLinuxATBeta`), in conflict with the stable one, `desc` marked.
pub fn beta_formula(stable: &str) -> Result<String> {
    let Some(class) = stable.lines().find_map(class_name) else {
        bail!("not a stable formula: no `class <Name> < Formula` line.");
    };
    if class.ends_with("ATBeta") {
        bail!("already a beta formula: {class}.");
    }
    let name = formula_name(class);
    let mut out = String::new();
    for line in stable.lines() {
        out.push_str(&beta_line(line, class, &name));
        out.push('\n');
    }
    Ok(out)
}

fn beta_line(line: &str, class: &str, name: &str) -> String {
    if class_name(line).is_some() {
        return format!(
            "class {class}ATBeta < Formula\n  conflicts_with \"{name}\", because: \"both install the acteble binary\""
        );
    }
    match line.trim_start().strip_prefix("desc \"") {
        Some(d) if d.ends_with('"') => format!("  desc \"{} (beta)\"", &d[..d.len() - 1]),
        _ => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    include!("brew_beta_formula_tests.rs");
}
