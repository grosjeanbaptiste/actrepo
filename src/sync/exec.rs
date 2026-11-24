//! Runs a plan: every step is attempted, each outcome is reported, one failure hides no other.
use super::plan::Step;
use crate::scaffold::setup::Exec;
use anyhow::{Result, bail};
use std::io::Write;

/// Pushes and pulls (fast-forward only) what the plan says; diverged and skipped steps run nothing.
/// Fails when any step failed or still needs a human, after all of them were tried.
pub fn execute(steps: &[Step], exec: &impl Exec, out: &mut impl Write) -> Result<()> {
    run_all(steps, exec, out, true)
}

/// Like `execute`, but a step that needs a human (diverged, skipped) is printed as a `note` and does
/// not fail the run: only a step that ran and failed does. For a session start, which must not
/// report an error every time an old branch has diverged.
pub fn execute_noting(steps: &[Step], exec: &impl Exec, out: &mut impl Write) -> Result<()> {
    run_all(steps, exec, out, false)
}

fn run_all(steps: &[Step], exec: &impl Exec, out: &mut impl Write, todo_fails: bool) -> Result<()> {
    if steps.is_empty() {
        writeln!(out, "nothing to sync")?;
        return Ok(());
    }
    let mut attention = 0;
    for s in steps {
        match run(s, exec) {
            Some(Ok(())) => writeln!(out, "ok   {}", s.line())?,
            Some(Err(e)) => {
                attention += 1;
                writeln!(out, "FAIL {}: {e}", s.line())?;
            }
            None if todo_fails => {
                attention += 1;
                writeln!(out, "TODO {}", s.line())?;
            }
            None => writeln!(out, "note {}", s.line())?,
        }
    }
    if attention > 0 {
        bail!("{attention} of {} steps need attention", steps.len());
    }
    Ok(())
}

/// `None` for a step that runs nothing.
fn run(step: &Step, exec: &impl Exec) -> Option<Result<()>> {
    let git = |repo: &std::path::Path, args: &[&str]| exec.run("git", args, Some(repo)).map(drop);
    match step {
        Step::Push { repo, branch, .. } => Some(git(repo, &["push", "origin", branch])),
        Step::Pull {
            repo,
            upstream,
            checked_out: true,
            ..
        } => Some(git(repo, &["merge", "--ff-only", upstream])),
        Step::Pull {
            repo,
            branch,
            upstream,
            ..
        } => {
            // The upstream may carry another name than the local branch (`origin/main`).
            let remote_branch = upstream
                .split_once('/')
                .map_or(upstream.as_str(), |(_, b)| b);
            Some(git(
                repo,
                &["fetch", "origin", &format!("{remote_branch}:{branch}")],
            ))
        }
        Step::Diverged { .. } | Step::Skip { .. } => None,
    }
}
