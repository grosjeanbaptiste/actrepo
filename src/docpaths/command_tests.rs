use super::*;
use std::fs;
use std::path::PathBuf;

fn ok<T, E: std::fmt::Display>(r: std::result::Result<T, E>) -> T {
    r.unwrap_or_else(|e| panic!("{e}"))
}

fn bad<T, E>(r: std::result::Result<T, E>) -> E {
    match r {
        Ok(_) => panic!("expected an error"),
        Err(e) => e,
    }
}

fn entries() -> Vec<Entry> {
    vec![Entry {
        name: "actrack".into(),
        path: PathBuf::from("/h/Source/Acteble/openacte/actrack"),
    }]
}

const STALE: &str = "| **actrack** | `~/Source/openacte/actrack` | x |\n| **usext** | `~/u` | y |\n";

fn file(dir: &tempfile::TempDir) -> PathBuf {
    let f = dir.path().join("eco.md");
    ok(fs::write(&f, STALE));
    f
}

#[test]
fn checking_a_stale_file_fails_naming_the_rows_and_writes_nothing() {
    let dir = ok(tempfile::tempdir());
    let f = file(&dir);
    let mut out = vec![];
    let err = bad(run(std::slice::from_ref(&f), false, &entries(), Path::new("/h"), &mut out));
    let said = ok(String::from_utf8(out));
    assert!(said.contains("actrack: ~/Source/openacte/actrack -> ~/Source/Acteble/openacte/actrack"));
    assert!(said.contains("not in the registry: usext"));
    assert!(err.to_string().contains("--write"), "{err}");
    assert_eq!(ok(fs::read_to_string(&f)), STALE);
}

#[test]
fn writing_fixes_the_file_and_succeeds() {
    let dir = ok(tempfile::tempdir());
    let f = file(&dir);
    ok(run(std::slice::from_ref(&f), true, &entries(), Path::new("/h"), &mut vec![]));
    let text = ok(fs::read_to_string(&f));
    assert!(text.contains("`~/Source/Acteble/openacte/actrack`"));
    assert!(text.contains("| **usext** | `~/u` |"));
}

#[test]
fn a_file_that_is_already_right_passes_the_check() {
    let dir = ok(tempfile::tempdir());
    let f = file(&dir);
    ok(run(std::slice::from_ref(&f), true, &entries(), Path::new("/h"), &mut vec![]));
    ok(run(std::slice::from_ref(&f), false, &entries(), Path::new("/h"), &mut vec![]));
}

#[test]
fn a_missing_file_is_an_error_naming_it() {
    let err = bad(run(&[PathBuf::from("/nope/eco.md")], false, &entries(), Path::new("/h"), &mut vec![]));
    assert!(err.to_string().contains("/nope/eco.md"), "{err}");
}
