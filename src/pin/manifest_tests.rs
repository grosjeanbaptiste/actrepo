use super::*;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn toml() -> String {
    format!(
        "[dependencies]\nactit = {{ git = \"ssh://x/Actit.git\", rev = \"{A}\" }}\nactautoma = {{ git = \"https://x/actautoma\", rev = \"{A}\" }}\nanyhow = \"1\"\n"
    )
}

#[test]
fn the_rev_of_a_git_dependency_is_read() {
    assert_eq!(rev_of(&toml(), "actit").ok().flatten().as_deref(), Some(A));
}

#[test]
fn a_dependency_that_is_absent_or_not_git_has_no_rev() {
    assert_eq!(rev_of(&toml(), "serde").ok(), Some(None));
    assert_eq!(rev_of(&toml(), "anyhow").ok(), Some(None));
}

#[test]
fn a_prefix_of_another_name_is_not_that_dependency() {
    assert_eq!(rev_of(&toml(), "act").ok(), Some(None));
}

#[test]
fn a_git_dependency_without_rev_is_an_error() {
    let t = "actit = { git = \"ssh://x\", branch = \"main\" }\n";
    assert!(rev_of(t, "actit").is_err());
}

#[test]
fn only_the_named_dependency_is_rewritten() {
    let out = with_rev(&toml(), "actit", B).ok().flatten().unwrap_or_default();
    assert_eq!(out, toml().replacen(A, B, 1));
}

#[test]
fn rewriting_an_absent_dependency_changes_nothing() {
    assert_eq!(with_rev(&toml(), "serde", B).ok(), Some(None));
}

#[test]
fn a_rev_that_is_not_a_full_hash_is_refused() {
    for bad in ["", "abc", "main", &"z".repeat(40)] {
        assert!(with_rev(&toml(), "actit", bad).is_err(), "{bad}");
    }
}
