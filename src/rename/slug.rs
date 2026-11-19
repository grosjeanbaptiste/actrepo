//! `repo rename OLD NEW`: a bare OLD gets its owner from the `origin` of the registered folder.
use crate::registry::file::{Entry, find};
use anyhow::{Result, bail};
use std::path::Path;

/// `OWNER` of a GitHub remote: `git@h:o/n.git`, `https://h/o/n`, `ssh://git@h/o/n.git`.
pub fn owner_of_url(url: &str) -> Option<&str> {
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => url.split_once(':')?.1,
    };
    let (owner, name) = path.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then_some(owner)
}

/// `OWNER/OLD`: `slug` itself when it has an owner, else the owner read from the origin of OLD's folder.
pub fn qualify(
    slug: &str,
    registry: &[Entry],
    origin: &dyn Fn(&Path) -> Result<String>,
) -> Result<String> {
    if slug.contains('/') {
        return Ok(slug.to_string());
    }
    let Some(dir) = find(registry, slug) else {
        bail!("{slug} is not registered: give OWNER/{slug}, or its folder with --dir");
    };
    let url = origin(&dir)?;
    match owner_of_url(&url) {
        Some(owner) => Ok(format!("{owner}/{slug}")),
        None => bail!(
            "no owner in the origin {url:?} of {}: give OWNER/{slug}",
            dir.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn registry() -> Vec<Entry> {
        vec![Entry {
            name: "servai".into(),
            path: PathBuf::from("/s/servai"),
        }]
    }

    fn origin(url: &'static str) -> impl Fn(&Path) -> Result<String> {
        move |_| Ok(url.to_string())
    }

    #[test]
    fn the_owner_is_read_from_every_remote_form() {
        for url in [
            "git@github.com:me/servai.git",
            "https://github.com/me/servai",
            "https://github.com/me/servai.git",
            "ssh://git@github.com/me/servai.git",
        ] {
            assert_eq!(owner_of_url(url), Some("me"), "{url}");
        }
        assert_eq!(owner_of_url("/local/path/servai"), None);
        assert_eq!(owner_of_url("https://github.com/servai"), None);
    }

    #[test]
    fn a_slug_with_an_owner_is_kept_as_typed() {
        let q = qualify(
            "you/servai",
            &registry(),
            &origin("git@github.com:me/servai.git"),
        );
        assert_eq!(q.unwrap_or_else(|e| panic!("{e}")), "you/servai");
    }

    #[test]
    fn a_bare_name_takes_the_owner_of_its_origin() {
        let q = qualify(
            "servai",
            &registry(),
            &origin("git@github.com:me/servai.git"),
        );
        assert_eq!(q.unwrap_or_else(|e| panic!("{e}")), "me/servai");
    }

    #[test]
    fn an_unregistered_bare_name_is_refused_naming_it() {
        let e = qualify("nope", &registry(), &origin("x"))
            .err()
            .map(|e| e.to_string());
        assert!(e.is_some_and(|e| e.contains("nope is not registered")));
    }

    #[test]
    fn an_origin_without_an_owner_is_an_error_not_a_guess() {
        let e = qualify("servai", &registry(), &origin("/local/servai"))
            .err()
            .map(|e| e.to_string());
        assert!(e.is_some_and(|e| e.contains("no owner in the origin")));
    }

    #[test]
    fn a_failing_origin_read_propagates() {
        let broken = |_: &Path| -> Result<String> { bail!("no origin remote") };
        let e = qualify("servai", &registry(), &broken)
            .err()
            .map(|e| e.to_string());
        assert!(e.is_some_and(|e| e.contains("no origin remote")));
    }
}
