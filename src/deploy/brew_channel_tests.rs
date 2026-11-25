use super::*;
use anyhow::anyhow;

#[test]
fn prod_is_stable_and_test_is_beta() {
    assert_eq!(Channel::of(Env::Prod), Channel::Stable);
    assert_eq!(Channel::of(Env::Test), Channel::Beta);
}

#[test]
fn the_name_is_what_the_bumper_expects() {
    assert_eq!(Channel::Stable.name(), "stable");
    assert_eq!(Channel::Beta.name(), "beta");
}

#[test]
fn stable_drops_the_build_suffix_and_never_asks_for_a_build_number() {
    let v = Channel::Stable.version("1.2.3+9", || Err(anyhow!("must not be called")));
    assert_eq!(v.unwrap_or_default(), "1.2.3");
    let v = Channel::Stable.version("1.2.3", || Err(anyhow!("must not be called")));
    assert_eq!(v.unwrap_or_default(), "1.2.3");
}

#[test]
fn beta_appends_the_build_number() {
    let v = Channel::Beta.version("1.2.3+9", || Ok(42));
    assert_eq!(v.unwrap_or_default(), "1.2.3-beta.42");
}

#[test]
fn a_beta_without_a_build_number_fails_loudly() {
    let e = Channel::Beta.version("1.2.3", || Err(anyhow!("cannot count the commits")));
    assert_eq!(e.err().map(|e| e.to_string()).as_deref(), Some("cannot count the commits"));
}

#[test]
fn only_the_beta_is_a_prerelease() {
    assert!(Channel::Beta.prerelease());
    assert!(!Channel::Stable.prerelease());
}

#[test]
fn titles_notes_and_commit_messages_differ_per_channel() {
    assert_eq!(Channel::Stable.title("Acteble", "1.2.3", "v1.2.3"), "Acteble 1.2.3");
    assert_eq!(Channel::Beta.title("Acteble", "1.2.3-beta.4", "v1.2.3-beta.4"), "v1.2.3-beta.4");
    assert_eq!(Channel::Stable.notes(), "Stable release");
    assert_eq!(Channel::Beta.notes(), "Beta build");
    assert_eq!(Channel::Stable.commit_message("Acteble", "1.2.3"), "chore: Acteble 1.2.3");
    assert_eq!(Channel::Beta.commit_message("Acteble", "1.2.3-beta.4"), "chore: Acteble beta 1.2.3-beta.4");
}
