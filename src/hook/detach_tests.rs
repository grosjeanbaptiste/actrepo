use super::*;
use std::time::{Duration, Instant};

fn wait_for(file: &Path, text: &str) -> String {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        let got = fs::read_to_string(file).unwrap_or_default();
        if got.contains(text) || Instant::now() > end {
            return got;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn the_log_follows_xdg_state_then_home() {
    let x = log_path(Some("/s"), Some("/h")).unwrap_or_default();
    assert_eq!(x, Path::new("/s/actrepo/hook.log"));
    let h = log_path(None, Some("/h")).unwrap_or_default();
    assert_eq!(h, Path::new("/h/.local/state/actrepo/hook.log"));
    let empty = log_path(Some(""), Some("/h")).unwrap_or_default();
    assert_eq!(empty, Path::new("/h/.local/state/actrepo/hook.log"));
}

#[test]
fn no_place_for_the_log_is_an_error() {
    let err = log_path(None, None)
        .err()
        .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("HOME"), "{err}");
}

#[test]
fn the_sync_runs_in_the_background_and_its_output_lands_in_the_log() {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let log = d.path().join("state/hook.log");
    detach(Path::new("/bin/echo"), d.path(), &log).unwrap_or_else(|e| panic!("{e}"));
    assert!(wait_for(&log, "hook sync").contains("hook sync"));
}

#[test]
fn a_missing_binary_is_an_error_naming_it() {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let err = detach(
        Path::new("/nonexistent/actrepo"),
        d.path(),
        &d.path().join("l.log"),
    )
    .err()
    .unwrap_or_else(|| panic!("expected an error"));
    assert!(err.to_string().contains("/nonexistent/actrepo"), "{err}");
}

#[test]
fn a_second_run_appends_to_the_log() {
    let d = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
    let log = d.path().join("hook.log");
    detach(Path::new("/bin/echo"), d.path(), &log).unwrap_or_else(|e| panic!("{e}"));
    wait_for(&log, "hook sync");
    detach(Path::new("/bin/echo"), d.path(), &log).unwrap_or_else(|e| panic!("{e}"));
    let end = Instant::now() + Duration::from_secs(10);
    while fs::read_to_string(&log)
        .unwrap_or_default()
        .matches("hook sync")
        .count()
        < 2
        && Instant::now() < end
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        fs::read_to_string(&log)
            .unwrap_or_default()
            .matches("hook sync")
            .count(),
        2
    );
}
