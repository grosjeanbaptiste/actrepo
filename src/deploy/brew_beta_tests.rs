use super::*;

const STABLE: &str = "cask \"acteble\" do\n  version \"0.0.0\"\n  desc \"Desktop client for the Acteble platform\"\n  app \"Acteble.app\"\nend\n";

#[test]
fn the_beta_version_is_the_base_version_plus_the_build() {
    assert_eq!(beta_version("1.2.3", 42), "1.2.3-beta.42");
}

#[test]
fn a_pubspec_build_suffix_is_dropped() {
    assert_eq!(beta_version("1.2.3+9", 42), "1.2.3-beta.42");
}

#[test]
fn each_beta_build_is_a_distinct_tag() {
    assert_ne!(tag(&beta_version("1.0.0", 1)), tag(&beta_version("1.0.0", 2)));
    assert_eq!(tag("1.2.3-beta.42"), "v1.2.3-beta.42");
}

#[test]
fn the_asset_is_named_as_the_cask_url_expects() {
    assert_eq!(asset_name("Acteble", "1.2.3-beta.42"), "Acteble-1.2.3-beta.42.dmg");
}

#[test]
fn the_beta_cask_is_named_and_conflicts_with_the_stable_one() {
    let beta = beta_cask(STABLE).unwrap_or_else(|e| panic!("{e}"));
    assert!(beta.starts_with("cask \"acteble@beta\" do\n  conflicts_with cask: \"acteble\"\n"));
}

#[test]
fn the_beta_cask_says_it_is_a_beta() {
    let beta = beta_cask(STABLE).unwrap_or_else(|e| panic!("{e}"));
    assert!(beta.contains("desc \"Desktop client for the Acteble platform (beta)\""));
}

#[test]
fn the_rest_of_the_cask_is_untouched() {
    let beta = beta_cask(STABLE).unwrap_or_else(|e| panic!("{e}"));
    assert!(beta.contains("  version \"0.0.0\"\n"));
    assert!(beta.ends_with("  app \"Acteble.app\"\nend\n"));
}

#[test]
fn a_text_that_is_not_a_cask_is_refused_by_name() {
    let e = beta_cask("formula \"x\" do\nend\n").err().map(|e| e.to_string());
    assert_eq!(e.as_deref(), Some("not a stable cask: no `cask \"<name>\" do` line."));
}

#[test]
fn a_beta_cask_is_not_made_into_a_beta_again() {
    let e = beta_cask("cask \"acteble@beta\" do\nend\n").err().map(|e| e.to_string());
    assert_eq!(e.as_deref(), Some("already a beta cask: acteble@beta."));
}
