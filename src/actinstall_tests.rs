use super::*;
use crate::scaffold::native::Process;
use std::cell::RefCell;
use std::time::{Duration, Instant};

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let v: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |k| v.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
}

#[test]
fn the_wait_defaults_to_ten_minutes_and_is_read_from_the_environment() {
    assert_eq!(wait_from_env(env(&[])).ok(), Some(600));
    assert_eq!(wait_from_env(env(&[("REPO_ACTINSTALL_WAIT", "30")])).ok(), Some(30));
}

#[test]
fn an_invalid_wait_fails_loudly_naming_the_variable() {
    for v in ["soon", "-1", "", "0"] {
        let why = wait_from_env(env(&[("REPO_ACTINSTALL_WAIT", v)])).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(why.contains("REPO_ACTINSTALL_WAIT"), "{v:?}: {why}");
    }
}

#[test]
fn a_program_that_takes_too_long_is_stopped_and_reported() {
    let start = Instant::now();
    let got = Process.run_within("sleep", &["5"], None, 1);
    assert!(start.elapsed() < Duration::from_secs(4), "not stopped: {:?}", start.elapsed());
    let why = got.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(why.contains("did not finish within 1s") && why.contains("sleep 5"), "{why}");
}

#[test]
fn a_program_that_finishes_in_time_succeeds_and_a_failing_one_still_fails() {
    assert!(Process.run_within("true", &[], None, 5).is_ok());
    let why = Process.run_within("false", &[], None, 5).err().map(|e| e.to_string()).unwrap_or_default();
    assert!(why.contains("exited with"), "{why}");
}

struct Slow(RefCell<Vec<u64>>);
impl Exec for Slow {
    fn run(&self, prog: &str, _: &[&str], _: Option<&Path>) -> Result<String> {
        if prog == "git" {
            return Ok("worktree /repo\n".into());
        }
        anyhow::bail!("actinstall must be asked through run_within")
    }
    fn run_within(&self, prog: &str, _: &[&str], _: Option<&Path>, secs: u64) -> Result<String> {
        self.0.borrow_mut().push(secs);
        anyhow::bail!("{prog} did not finish within {secs}s")
    }
}

#[test]
fn refresh_bounds_actinstall_and_reports_the_timeout_as_a_warning() {
    let slow = Slow(RefCell::new(vec![]));
    let mut err = Vec::new();
    let done = refresh(&slow, &slow, Path::new("."), &mut err);
    assert_eq!(done.ok(), Some(false));
    assert_eq!(*slow.0.borrow(), [600]);
    let err = String::from_utf8_lossy(&err);
    assert!(err.contains("warning: actinstall update failed for /repo") && err.contains("did not finish within 600s"), "{err}");
}
