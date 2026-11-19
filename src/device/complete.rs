//! `repo __complete WORDS…`: what the shell may offer at the cursor (last word = the one being typed).
use crate::registry::file::Entry;
use anyhow::Result;

/// `id<TAB>name` per candidate, so a shell can show the name and insert the id.
pub fn candidates(
    words: &[String],
    entries: &[Entry],
    devices: &dyn Fn() -> Result<Vec<String>>,
) -> Result<Vec<String>> {
    let (typing, before) = match words.split_last() {
        Some((last, before)) => (last.as_str(), before),
        None => ("", words),
    };
    let starts = |s: &str| s.to_lowercase().starts_with(&typing.to_lowercase());
    Ok(match before {
        [] => entries
            .iter()
            .map(|e| e.name.clone())
            .filter(|n| starts(n))
            .collect(),
        [name] if entries.iter().any(|e| &e.name == name) => ["run", "install"]
            .into_iter()
            .filter(|v| starts(v))
            .map(String::from)
            .collect(),
        [name, verb]
            if matches!(verb.as_str(), "run" | "install")
                && entries.iter().any(|e| &e.name == name) =>
        {
            devices()?
                .into_iter()
                .filter(|d| starts(d) || d.to_lowercase().contains(&typing.to_lowercase()))
                .collect()
        }
        _ => vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::path::PathBuf;

    fn words(s: &str) -> Vec<String> {
        s.split(' ').map(String::from).collect()
    }

    fn entries() -> Vec<Entry> {
        ["alpha", "actrepo", "beta"]
            .map(|n| Entry {
                name: n.into(),
                path: PathBuf::from(format!("/s/{n}")),
            })
            .to_vec()
    }

    fn fleet() -> Result<Vec<String>> {
        Ok(vec!["A1\tiPhone 15".into(), "R59X\tPixel 7".into()])
    }

    fn got(line: &str) -> Vec<String> {
        candidates(&words(line), &entries(), &fleet).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn the_first_word_completes_registered_names_by_prefix() {
        assert_eq!(got("a"), ["alpha", "actrepo"]);
        assert_eq!(got(""), ["alpha", "actrepo", "beta"]);
    }

    #[test]
    fn after_a_project_the_verbs_are_offered() {
        assert_eq!(got("alpha "), ["run", "install"]);
        assert_eq!(got("alpha i"), ["install"]);
    }

    #[test]
    fn after_run_or_install_the_devices_are_offered_and_filtered() {
        assert_eq!(got("alpha run "), ["A1\tiPhone 15", "R59X\tPixel 7"]);
        assert_eq!(got("alpha install pix"), ["R59X\tPixel 7"]);
    }

    #[test]
    fn nothing_is_offered_after_the_device_or_after_an_unknown_verb() {
        assert!(got("alpha run A1 ").is_empty());
        assert!(got("alpha fly ").is_empty());
        assert!(got("nope run ").is_empty());
    }

    #[test]
    fn devices_are_not_listed_until_asked() {
        let asked = Cell::new(false);
        let probe = || {
            asked.set(true);
            fleet()
        };
        let _ = candidates(&words("alpha "), &entries(), &probe);
        assert!(!asked.get());
    }

    #[test]
    fn a_failing_device_listing_is_an_error_not_an_empty_offer() {
        let broken = || anyhow::bail!("adb: daemon");
        let e = candidates(&words("alpha run "), &entries(), &broken);
        assert!(
            e.err()
                .is_some_and(|e| e.to_string().contains("adb: daemon"))
        );
    }
}
