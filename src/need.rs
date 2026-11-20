//! `actrepo new --need`: ask actit whether a tool already covers the need before creating a
//! repository. Only a tool whose licence allows reuse stops the creation.
use actit::cli::Outcome;
use anyhow::{Result, anyhow, bail};
use std::io::Write;

/// A tool actit confirmed for the need, with its licence verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub id: String,
    pub registry: String,
    pub url: String,
    /// `MIT OR Apache-2.0`, or `none declared`.
    pub license: String,
    /// `permissive`, `strong-copyleft`, `unknown`...
    pub class: String,
}

/// What actit found for a need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reuse {
    /// Confirmed tools whose licence allows reuse.
    pub reusable: Vec<Tool>,
    /// Confirmed tools whose licence does not (copyleft outside the policy, unknown...).
    pub blocked: Vec<Tool>,
    /// Candidates actit could not read, with the reason: never dropped silently.
    pub unreadable: Vec<String>,
}

/// Port: does a tool already exist for this need? (actit: indexer + kag)
pub trait ExistingTools {
    fn find(&self, need: &str) -> Result<Reuse>;
}

/// Adapter: actit run in-process through its library entry point.
pub struct ActitTools;

impl ExistingTools for ActitTools {
    fn find(&self, need: &str) -> Result<Reuse> {
        let env = |key: &str| std::env::var(key).ok();
        let outcome =
            actit::cli::search(&[need.to_owned()], &env).map_err(|e| anyhow!("actit: {e}"))?;
        Ok(reuse_of(&outcome))
    }
}

fn tool_of(finding: &actit::model::Finding, check: &actit::license::Check) -> Tool {
    let c = &finding.candidate;
    Tool {
        id: c.id.clone(),
        registry: c.registry.clone(),
        url: c.url.clone(),
        license: check
            .expression
            .clone()
            .unwrap_or_else(|| "none declared".into()),
        class: check.class.wire().to_owned(),
    }
}

/// The tools of an actit outcome, split by what their licence allows.
pub fn reuse_of(outcome: &Outcome) -> Reuse {
    let mut reuse = Reuse::default();
    for (finding, check) in outcome.report.findings.iter().zip(&outcome.licenses) {
        let tool = tool_of(finding, check);
        if check.allowed {
            reuse.reusable.push(tool);
        } else {
            reuse.blocked.push(tool);
        }
    }
    for (candidate, why) in &outcome.report.failures {
        reuse
            .unreadable
            .push(format!("{} ({}): {why}", candidate.id, candidate.registry));
    }
    reuse
}

fn describe(t: &Tool) -> String {
    format!(
        "{} ({}) {} licence {} ({})",
        t.id, t.registry, t.url, t.license, t.class
    )
}

/// Stops with an error when a reusable tool exists, unless `force`; says what it found on `err`.
/// A failing lookup is always an error: an unchecked need is not a free one.
pub fn check(
    need: &str,
    force: bool,
    tools: &dyn ExistingTools,
    err: &mut dyn Write,
) -> Result<()> {
    let reuse = tools.find(need)?;
    for why in &reuse.unreadable {
        writeln!(err, "warn: need check could not read {why}")?;
    }
    for t in &reuse.blocked {
        writeln!(err, "found but not reusable: {}", describe(t))?;
    }
    for t in &reuse.reusable {
        writeln!(
            err,
            "warn: a reusable tool already exists for this need: {}",
            describe(t)
        )?;
    }
    if reuse.reusable.is_empty() || force {
        return Ok(());
    }
    let names: Vec<String> = reuse.reusable.iter().map(describe).collect();
    bail!(
        "a tool already covers this need: {}; use --force to create the repository anyway",
        names.join("; ")
    )
}

#[cfg(test)]
mod tests {
    include!("need_tests.rs");
}
