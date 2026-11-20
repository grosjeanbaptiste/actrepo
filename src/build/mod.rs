//! `repo <dir> build [--clean] [--release] [--target T] [source]`: the artifact of a folder, through actmake.
use crate::kind::detect;
use crate::registry::file::{path, read};
use crate::run::resolve;
use actmake::{Options, System, build};
use anyhow::{Result, bail};

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub name: String,
    pub opts: Options,
}

/// `<name> build ...`: the shape that is ours, not a `dotfiles project` command.
pub fn is_build_request(args: &[String]) -> bool {
    args.get(1).is_some_and(|v| v == "build")
}

/// `<name> build [--clean] [SOURCE]`; anything else is an error naming what was typed.
pub fn parse(args: &[String]) -> Result<Request> {
    let [name, verb, rest @ ..] = args else {
        bail!(
            "usage: repo <dir> build [--clean] [--release] [--target T] [source] (got: {})",
            args.join(" ")
        );
    };
    if verb != "build" {
        bail!(
            "unknown command `{name} {verb}`; expected: repo {name} build [--clean] [--release] [--target T] [source]"
        );
    }
    let mut opts = Options::default();
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match word.as_str() {
            "--clean" => opts.clean = true,
            "--release" => opts.release = true,
            "--target" => {
                let value = words
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--target needs a value"))?;
                opts.target = Some(value.clone());
            }
            flag if flag.starts_with('-') => bail!("unknown option {flag}"),
            _ if opts.main.is_some() => bail!("one source file only; got a second: {word}"),
            _ => opts.main = Some(word.clone()),
        }
    }
    Ok(Request {
        name: name.clone(),
        opts,
    })
}

pub fn command(args: &[String]) -> Result<()> {
    let req = parse(args)?;
    let entries = read(&path(|k| std::env::var(k).ok())?)?;
    let dir = resolve(&req.name, &entries)?;
    let kind = detect(&dir)?;
    let Some(engine) = kind.engine() else {
        bail!(
            "{} is a {kind:?} project: no build engine for it yet",
            req.name
        );
    };
    build(&System, &dir, engine, &req.opts).map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod request_tests;
