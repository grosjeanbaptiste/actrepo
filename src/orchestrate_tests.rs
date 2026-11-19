use super::*;

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(ToString::to_string).collect()
}

fn no_env(_: &str) -> Option<String> {
    None
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8_lossy(&out).into_owned()
}

#[test]
fn actname_generates_names_in_process() {
    let mut out = Vec::new();
    let a = args(&["generate", "--prefix", "act", "--domain", "architecture plans", "--limit", "2"]);
    actname(&a, &mut out).unwrap_or_else(|e| panic!("{e}"));
    let out = text(out);
    assert_eq!(out.lines().count(), 3, "{out}");
    assert!(out.starts_with("NAME"), "{out}");
}

#[test]
fn an_actname_error_keeps_its_message() {
    let err = actname(&args(&["generate", "--zap", "1"]), &mut Vec::new()).err();
    assert!(err.is_some_and(|e| e.to_string().contains("unknown option: --zap")));
}

#[test]
fn an_actit_failure_is_an_error_not_a_verdict() {
    let mut out = Vec::new();
    let err = actit(&args(&[]), &no_env, &mut out).err();
    assert!(err.is_some_and(|e| e.to_string().contains("the need is empty")));
    assert!(out.is_empty());
}

#[test]
fn actautoma_dry_run_lists_the_renames_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let old = dir.path().join("old");
    std::fs::create_dir(&old).unwrap_or_else(|e| panic!("{e}"));
    let env = |k: &str| (k == "HOME").then(|| dir.path().display().to_string());
    let a = args(&["rename-repo", "me/old", "new", "--dir", &old.display().to_string(), "--dry-run"]);
    let mut out = Vec::new();
    actautoma(&a, dir.path(), &env, &mut out).unwrap_or_else(|e| panic!("{e}"));
    let out = text(out);
    assert!(out.contains("me/old -> me/new") && out.contains("dry run"), "{out}");
    assert!(old.is_dir() && !dir.path().join("new").exists());
}
