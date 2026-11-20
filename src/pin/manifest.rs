//! Read and rewrite the `rev` of one git dependency in a Cargo.toml, keeping every other byte.
use anyhow::{Result, bail};

/// The line of `[dependencies]`-style tables declaring `dep = { … rev = "…" … }`.
fn line_of<'a>(toml: &'a str, dep: &str) -> Option<&'a str> {
    toml.lines().find(|l| {
        let t = l.trim_start();
        t.strip_prefix(dep)
            .is_some_and(|r| r.trim_start().starts_with('=') && t.contains("git"))
    })
}

fn rev_span(line: &str) -> Option<(usize, usize)> {
    let at = line.find("rev")?;
    let open = at + line[at..].find('"')? + 1;
    Some((open, open + line[open..].find('"')?))
}

/// The pinned revision; `None` when `dep` is not a git dependency here.
pub fn rev_of(toml: &str, dep: &str) -> Result<Option<String>> {
    let Some(line) = line_of(toml, dep) else {
        return Ok(None);
    };
    match rev_span(line) {
        Some((a, b)) => Ok(Some(line[a..b].to_string())),
        None => bail!("{dep} is a git dependency without a rev: nothing to pin"),
    }
}

/// The text with `dep` pinned to `rev`; `None` when `dep` is not a git dependency here.
pub fn with_rev(toml: &str, dep: &str, rev: &str) -> Result<Option<String>> {
    if rev.len() != 40 || !rev.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("{rev:?} is not a full 40-character commit hash");
    }
    if rev_of(toml, dep)?.is_none() {
        return Ok(None);
    };
    let line = line_of(toml, dep).unwrap_or_default(); // rev_of found it
    let (a, b) = rev_span(line).unwrap_or_default(); // rev_of found it
    let new = format!("{}{rev}{}", &line[..a], &line[b..]);
    Ok(Some(toml.replacen(line, &new, 1)))
}

#[cfg(test)]
mod tests {
    include!("manifest_tests.rs");
}
