//! The real ports of `actrepo pr`: the platform (through actautoma) and git on the process, stdin/stdout, the clock.
use super::{create, merge, verbs};
use crate::scaffold::native::{Process, Quiet};
use actautoma::pulls::PrFilter;
use actautoma::{gh::Gh, process::ProcessRunner};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;

pub fn run_create(opts: &create::Opts) -> Result<()> {
    let mut out = std::io::stdout().lock();
    create::run(
        opts,
        &Quiet,
        &Gh::new(ProcessRunner),
        Path::new("."),
        &mut std::io::stdin().lock(),
        &mut out,
    )
}

pub fn run_merge(opts: &merge::Opts) -> Result<()> {
    let wait = merge::wait_from_env(|k| std::env::var(k).ok())?;
    let sleep = |s| std::thread::sleep(Duration::from_secs(s));
    let (mut out, mut err) = (std::io::stdout().lock(), std::io::stderr().lock());
    let input = &mut std::io::stdin().lock();
    merge::run(
        opts,
        &wait,
        &Gh::new(ProcessRunner),
        &Quiet,
        &Process,
        &sleep,
        Path::new("."),
        input,
        &mut out,
        &mut err,
    )
}

fn forge() -> Gh<ProcessRunner> {
    Gh::new(ProcessRunner)
}

pub fn run_view(number: Option<u64>) -> Result<()> {
    verbs::view(&forge(), number, &mut std::io::stdout().lock())
}

pub fn run_list(filter: PrFilter) -> Result<()> {
    verbs::list(&forge(), filter, &mut std::io::stdout().lock())
}

pub fn run_retarget(number: u64, base: &str) -> Result<()> {
    verbs::retarget(&forge(), number, base, &mut std::io::stdout().lock())
}

pub fn run_close(number: u64, comment: Option<&str>) -> Result<()> {
    verbs::close(&forge(), number, comment, &mut std::io::stdout().lock())
}
