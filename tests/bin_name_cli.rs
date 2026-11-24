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
fn the_old_repo_name_is_no_longer_a_binary() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!root.join("src/bin/repo.rs").exists());
}
