use super::*;
use anyhow::anyhow;
use std::cell::RefCell;

fn e(name: &str, path: &str) -> Entry {
    Entry { name: name.into(), path: path.into() }
}

fn rows() -> Vec<Entry> {
    vec![e("a", "/s/a"), e("old", "/s/old")]
}

/// What the registry file ends up holding, and what `settle` answered.
struct Run {
    written: RefCell<Vec<Vec<Entry>>>,
}

impl Run {
    fn new() -> Self {
        Run { written: RefCell::new(vec![]) }
    }
    fn settle(
        &self,
        entries: &[Entry],
        executed: Result<()>,
        moved: bool,
        both: bool,
        write_fails: bool,
    ) -> Result<Vec<String>> {
        settle(
            entries,
            "old",
            "new",
            Path::new("/s/new"),
            executed,
            &|p| match (p == Path::new("/s/old"), p == Path::new("/s/new")) {
                (true, _) => both || !moved,
                (_, true) => both || moved,
                _ => false,
            },
            &mut |rows| {
                if write_fails {
                    return Err(anyhow!("disk full"));
                }
                self.written.borrow_mut().push(rows.to_vec());
                Ok(())
            },
        )
    }
}

fn moved_rows() -> Vec<Entry> {
    vec![e("a", "/s/a"), e("new", "/s/new")]
}

#[test]
fn a_clean_rename_writes_the_moved_row_and_says_nothing() {
    let run = Run::new();
    let warnings = run.settle(&rows(), Ok(()), true, false, false);
    assert_eq!(warnings.map_err(|e| e.to_string()), Ok(vec![]));
    assert_eq!(*run.written.borrow(), [moved_rows()]);
}

#[test]
fn a_clean_rename_of_an_unregistered_name_warns_and_writes_nothing() {
    let run = Run::new();
    let warnings = run.settle(&[e("a", "/s/a")], Ok(()), true, false, false);
    let warnings = warnings.unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("old was not in the registry"), "{warnings:?}");
    assert!(run.written.borrow().is_empty());
}

#[test]
fn a_clean_rename_whose_registry_cannot_be_written_says_everything_else_was_renamed() {
    let run = Run::new();
    let error = run.settle(&rows(), Ok(()), true, false, true);
    let message = error.err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(message.contains("renamed everywhere, but the registry"), "{message}");
    assert!(message.contains("disk full"), "{message}");
}

#[test]
fn a_failure_after_the_folder_moved_writes_the_moved_row_and_still_fails() {
    let run = Run::new();
    let error = run.settle(&rows(), Err(anyhow!("git failed")), true, false, false);
    let message = error.err().map(|e| e.to_string()).unwrap_or_default();
    assert_eq!(message, "git failed");
    assert_eq!(*run.written.borrow(), [moved_rows()]);
}

#[test]
fn a_failure_before_the_folder_moved_leaves_the_registry_alone() {
    let run = Run::new();
    let error = run.settle(&rows(), Err(anyhow!("platform refused")), false, false, false);
    assert_eq!(error.err().map(|e| e.to_string()), Some("platform refused".into()));
    assert!(run.written.borrow().is_empty());
}

#[test]
fn a_failure_with_the_folder_under_both_names_leaves_the_registry_alone() {
    let run = Run::new();
    let error = run.settle(&rows(), Err(anyhow!("half copied")), false, true, false);
    assert_eq!(error.err().map(|e| e.to_string()), Some("half copied".into()));
    assert!(run.written.borrow().is_empty());
}

#[test]
fn a_failure_for_an_unregistered_name_writes_nothing() {
    let run = Run::new();
    let error = run.settle(&[e("a", "/s/a")], Err(anyhow!("git failed")), true, false, false);
    assert_eq!(error.err().map(|e| e.to_string()), Some("git failed".into()));
    assert!(run.written.borrow().is_empty());
}

#[test]
fn a_failure_whose_registry_cannot_be_written_names_both_failures() {
    let run = Run::new();
    let error = run.settle(&rows(), Err(anyhow!("git failed")), true, false, true);
    let message = error.err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(message.contains("git failed"), "{message}");
    assert!(message.contains("was not updated"), "{message}");
    assert!(message.contains("disk full"), "{message}");
}

#[test]
fn the_other_rows_are_never_touched() {
    let run = Run::new();
    let many = vec![e("a", "/s/a"), e("old", "/s/old"), e("z", "/s/z")];
    let _ = run.settle(&many, Err(anyhow!("git failed")), true, false, false);
    assert_eq!(
        *run.written.borrow(),
        [vec![e("a", "/s/a"), e("new", "/s/new"), e("z", "/s/z")]]
    );
}
