//! `actrepo actit|actname|actautoma ...`: the three tools run in-process through their own
//! library entry points, with their own arguments, answers and exit codes.
use actautoma::{gh::Gh, process::ProcessRunner};
use anyhow::{Error, Result, anyhow};
use std::io::Write;
use std::path::Path;

/// `actit <args>`: the exit code of its verdict (0, 1); a failure is an error.
pub fn actit(
    args: &[String],
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
) -> Result<i32> {
    actit::cli::dispatch(args, env, out).map_err(|e| anyhow!("actit: {e}"))
}

/// `actname <args>`.
pub fn actname(args: &[String], out: &mut dyn Write) -> Result<()> {
    actname::cli::dispatch(args, out).map_err(Error::msg)
}

/// `actautoma <args>` run in `cwd`, on GitHub through `gh`.
pub fn actautoma(
    args: &[String],
    cwd: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
) -> Result<()> {
    let forge = Gh::new(ProcessRunner);
    actautoma::cli::dispatch(args, cwd, env, &forge, &ProcessRunner, out)
        .map_err(|e| anyhow!("actautoma: {e}"))
}

#[cfg(test)]
mod tests {
    include!("orchestrate_tests.rs");
}
