use super::*;
use std::path::PathBuf;

fn entry(name: &str, path: &str) -> Entry {
    Entry {
        name: name.into(),
        path: PathBuf::from(path),
    }
}

fn run(text: &str) -> Fixed {
    let entries = [
        entry("actrack", "/h/Source/Acteble/openacte/actrack"),
        entry("Actit", "/h/Source/Acteble/openacte/Actit"),
    ];
    fix(text, &entries, Path::new("/h"))
}

#[test]
fn a_stale_path_becomes_the_registry_path_written_with_a_tilde() {
    let f = run("| **actrack** | `~/Source/openacte/actrack` | owns |\n");
    assert_eq!(f.text, "| **actrack** | `~/Source/Acteble/openacte/actrack` | owns |\n");
    assert_eq!(
        f.changed,
        ["actrack: ~/Source/openacte/actrack -> ~/Source/Acteble/openacte/actrack"]
    );
}

#[test]
fn text_after_the_path_and_the_other_cells_are_kept() {
    let f = run("| **actrack** | `~/old/actrack` (deck, lab) | a | b |\n");
    assert_eq!(
        f.text,
        "| **actrack** | `~/Source/Acteble/openacte/actrack` (deck, lab) | a | b |\n"
    );
}

#[test]
fn a_correct_path_changes_nothing() {
    let t = "| **actrack** | `~/Source/Acteble/openacte/actrack` | owns |\n";
    let f = run(t);
    assert_eq!(f.text, t);
    assert!(f.changed.is_empty() && f.unknown.is_empty());
}

#[test]
fn the_name_matches_whatever_its_case() {
    let f = run("| **actit** | `~/old/Actit` | x |\n");
    assert!(f.text.contains("`~/Source/Acteble/openacte/Actit`"));
}

#[test]
fn a_row_the_registry_does_not_know_is_reported_and_left_alone() {
    let t = "| **usext** | `~/Source/usext` | x |\n";
    let f = run(t);
    assert_eq!(f.text, t);
    assert_eq!(f.unknown, ["usext"]);
}

#[test]
fn lines_that_are_not_repository_rows_are_untouched() {
    let t = "# Title\n\n| Repository | Path |\n| --- | --- |\nplain `~/old/actrack` text\n";
    let f = run(t);
    assert_eq!(f.text, t);
    assert!(f.unknown.is_empty());
}

#[test]
fn a_row_whose_path_is_not_a_tilde_path_is_left_alone() {
    let t = "| **actrack** | see the repo | x |\n";
    assert_eq!(run(t).text, t);
}

#[test]
fn a_path_outside_home_is_written_absolute() {
    let entries = [entry("tool", "/opt/tool")];
    let f = fix("| **tool** | `~/x/tool` | y |\n", &entries, Path::new("/h"));
    assert_eq!(f.text, "| **tool** | `/opt/tool` | y |\n");
}
