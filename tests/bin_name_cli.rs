use std::process::Command;

#[test]
fn the_binary_is_named_actrepo_and_says_so_in_its_usage() {
    let o = Command::new(env!("CARGO_BIN_EXE_actrepo"))
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("Usage: actrepo "), "{out}");
}

#[test]
fn the_repo_alias_runs_the_same_tool() {
    let o = Command::new(env!("CARGO_BIN_EXE_repo"))
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("Usage: actrepo "), "{out}");
}
