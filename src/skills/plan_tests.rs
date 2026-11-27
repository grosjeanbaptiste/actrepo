#![allow(clippy::unwrap_used)]
use super::plan::{Step, apply, only, plan};
use crate::registry::file::Entry;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use tempfile::TempDir;

fn repo(root: &Path, name: &str, skills: &[&str]) -> Entry {
    let path = root.join(name);
    for s in skills {
        let dir = path.join(".claude/skills").join(s);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    Entry {
        name: name.into(),
        path,
    }
}

#[test]
fn a_skill_not_yet_linked_is_to_be_linked() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "actusext", &["actusext"]);
    let dest = t.path().join("skills");
    let steps = plan(&[e], &dest).unwrap();
    assert_eq!(
        steps,
        vec![Step::Link {
            name: "actusext".into(),
            target: t.path().join("actusext/.claude/skills/actusext"),
            link: dest.join("actusext"),
        }]
    );
}

#[test]
fn a_repository_without_skills_or_without_folder_gives_nothing() {
    let t = TempDir::new().unwrap();
    let a = repo(t.path(), "a", &[]);
    let gone = Entry {
        name: "gone".into(),
        path: t.path().join("gone"),
    };
    assert!(plan(&[a, gone], &t.path().join("s")).unwrap().is_empty());
}

#[test]
fn a_folder_without_skill_md_is_not_a_skill() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "a", &[]);
    fs::create_dir_all(e.path.join(".claude/skills/notes")).unwrap();
    assert!(plan(&[e], &t.path().join("s")).unwrap().is_empty());
}

#[test]
fn applying_creates_the_folder_and_the_link_and_a_second_plan_says_linked() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "actusext", &["actusext"]);
    let dest = t.path().join("deep/skills");
    apply(&plan(std::slice::from_ref(&e), &dest).unwrap()).unwrap();
    assert_eq!(
        fs::read_link(dest.join("actusext")).unwrap(),
        e.path.join(".claude/skills/actusext")
    );
    assert_eq!(
        plan(&[e], &dest).unwrap(),
        vec![Step::Linked {
            name: "actusext".into()
        }]
    );
}

#[test]
fn a_real_folder_in_the_way_is_a_conflict_and_is_left_alone() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "actusext", &["actusext"]);
    let dest = t.path().join("skills");
    fs::create_dir_all(dest.join("actusext")).unwrap();
    fs::write(dest.join("actusext/SKILL.md"), "hand made").unwrap();
    let steps = plan(&[e], &dest).unwrap();
    assert!(matches!(&steps[0], Step::Conflict { name, reason }
        if name == "actusext" && reason.contains("not a link")));
    apply(&steps).unwrap_err();
    assert_eq!(
        fs::read_to_string(dest.join("actusext/SKILL.md")).unwrap(),
        "hand made"
    );
}

#[test]
fn a_link_to_somewhere_else_is_a_conflict_naming_where() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "actusext", &["actusext"]);
    let dest = t.path().join("skills");
    fs::create_dir_all(&dest).unwrap();
    symlink(t.path().join("other"), dest.join("actusext")).unwrap();
    let steps = plan(&[e], &dest).unwrap();
    assert!(matches!(&steps[0], Step::Conflict { reason, .. }
        if reason.contains(t.path().join("other").to_str().unwrap())));
}

#[test]
fn two_repositories_shipping_one_name_conflict_and_neither_is_linked() {
    let t = TempDir::new().unwrap();
    let a = repo(t.path(), "a", &["same"]);
    let b = repo(t.path(), "b", &["same"]);
    let dest = t.path().join("skills");
    let steps = plan(&[a, b], &dest).unwrap();
    assert_eq!(steps.len(), 1);
    assert!(matches!(&steps[0], Step::Conflict { name, reason }
        if name == "same" && reason.contains("a") && reason.contains("b")));
    apply(&steps).unwrap_err();
    assert!(!dest.join("same").exists());
}

#[test]
fn applying_links_what_it_can_before_reporting_a_conflict() {
    let t = TempDir::new().unwrap();
    let a = repo(t.path(), "a", &["one", "two"]);
    let dest = t.path().join("skills");
    fs::create_dir_all(dest.join("one")).unwrap();
    let err = apply(&plan(&[a], &dest).unwrap()).unwrap_err();
    assert!(err.to_string().contains("one"));
    assert!(fs::read_link(dest.join("two")).is_ok());
}

#[test]
fn an_unreadable_skills_folder_is_an_error_naming_it() {
    let t = TempDir::new().unwrap();
    let e = repo(t.path(), "a", &[]);
    fs::write(e.path.join(".claude/skills"), "a file, not a folder").unwrap_or_else(|_| {
        fs::create_dir_all(e.path.join(".claude")).unwrap();
        fs::write(e.path.join(".claude/skills"), "a file").unwrap();
    });
    let err = plan(&[e], &t.path().join("s")).unwrap_err();
    assert!(err.to_string().contains(".claude/skills"));
}

#[test]
fn only_keeps_the_named_skills() {
    let t = TempDir::new().unwrap();
    let a = repo(t.path(), "a", &["one", "two"]);
    let steps = plan(&[a], &t.path().join("s")).unwrap();
    let kept = only(steps, &["two".to_string()]).unwrap();
    assert!(matches!(&kept[..], [Step::Link { name, .. }] if name == "two"));
}

#[test]
fn only_a_name_no_repository_ships_is_an_error_naming_it() {
    let t = TempDir::new().unwrap();
    let a = repo(t.path(), "a", &["one"]);
    let steps = plan(&[a], &t.path().join("s")).unwrap();
    let err = only(steps, &["nope".to_string()]).unwrap_err();
    assert!(err.to_string().contains("nope"));
}
