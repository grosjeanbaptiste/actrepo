//! The words after `repo`, read as a device request.
use actdevice::plan::{Action, Mode};
use anyhow::{Result, bail};

#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub name: String,
    pub action: Action,
    /// None: list the devices instead.
    pub device: Option<String>,
    pub mode: Mode,
}

/// `<name> run|install ...`: the shape that is ours, not a `dotfiles project` command.
pub fn is_device_request(args: &[String]) -> bool {
    matches!(args.get(1).map(String::as_str), Some("run" | "install"))
}

/// `<name> run|install [DEVICE] [--release]`; anything else is an error naming what was typed.
pub fn parse(args: &[String]) -> Result<Request> {
    let [name, verb, rest @ ..] = args else {
        bail!(
            "usage: repo <dir> run|install [DEVICE] [--release] (got: {})",
            args.join(" ")
        );
    };
    let action = match verb.as_str() {
        "run" => Action::Run,
        "install" => Action::Install,
        other => {
            bail!("unknown command `{name} {other}`; expected: repo {name} run|install [DEVICE]")
        }
    };
    let mut mode = Mode::Debug;
    let mut device = None;
    for word in rest {
        match word.as_str() {
            "--release" => mode = Mode::Release,
            flag if flag.starts_with('-') => bail!("unknown option {flag}"),
            _ if device.is_some() => bail!("one device only; got a second: {word}"),
            _ => device = Some(word.clone()),
        }
    }
    Ok(Request {
        name: name.clone(),
        action,
        device,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn only_name_then_run_or_install_is_ours() {
        assert!(is_device_request(&words("app run pixel")));
        assert!(is_device_request(&words("app install")));
        assert!(!is_device_request(&words("status app")));
        assert!(!is_device_request(&words("list")));
    }

    #[test]
    fn a_device_and_release_are_read() {
        let r = parse(&words("app run pixel --release")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            r,
            Request {
                name: "app".into(),
                action: Action::Run,
                device: Some("pixel".into()),
                mode: Mode::Release
            }
        );
    }

    #[test]
    fn install_without_a_device_lists_them() {
        let r = parse(&words("app install")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((r.action, r.device), (Action::Install, None));
    }

    #[test]
    fn an_unknown_verb_names_the_command_typed() {
        let e = parse(&words("app fly x"))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(e.contains("unknown command `app fly`"), "{e}");
    }

    #[test]
    fn a_second_device_or_an_unknown_flag_is_refused() {
        assert!(parse(&words("app run a b")).is_err());
        assert!(parse(&words("app run --fast")).is_err());
        assert!(parse(&words("app")).is_err());
    }
}
