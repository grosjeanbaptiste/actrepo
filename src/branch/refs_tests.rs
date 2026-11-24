use super::*;

fn r(line: &str, github: bool) -> Option<String> {
    rewritten(line, "master", "main", github)
}

#[test]
fn a_word_is_matched_whole_so_masterpiece_is_not_a_reference() {
    assert!(mentions("branches: [master]", "master"));
    assert!(mentions("git checkout master", "master"));
    assert!(!mentions("a masterpiece", "master"));
    assert!(!mentions("grandmaster_x", "master"));
}

#[test]
fn under_github_every_whole_word_is_the_branch() {
    assert_eq!(r("    branches: [master]", true).as_deref(), Some("    branches: [main]"));
    assert_eq!(r("if: github.ref == 'refs/heads/master'", true).as_deref(), Some("if: github.ref == 'refs/heads/main'"));
    assert_eq!(r("a masterpiece", true), None);
}

#[test]
fn elsewhere_only_forms_that_name_the_branch_are_rewritten() {
    assert_eq!(r("git push origin/master", false).as_deref(), Some("git push origin/main"));
    assert_eq!(r("pushed to the `master` branch", false).as_deref(), Some("pushed to the `main` branch"));
    assert_eq!(r("on branch master only", false).as_deref(), Some("on branch main only"));
    assert_eq!(r("git diff heads/master", false).as_deref(), Some("git diff heads/main"));
}

#[test]
fn prose_that_only_uses_the_word_is_left_alone() {
    assert_eq!(r("Mémoire de master en informatique", false), None);
    assert_eq!(r("a master thesis", false), None);
    assert_eq!(r("master", false), None);
}

#[test]
fn every_occurrence_on_a_line_is_rewritten() {
    assert_eq!(r("[master, release/master]", true).as_deref(), Some("[main, release/main]"));
    assert_eq!(r("origin/master and origin/master", false).as_deref(), Some("origin/main and origin/main"));
}
