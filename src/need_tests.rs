use super::*;
use actit::license::{check as licence, default_allow, Table};
use actit::model::{Candidate, Evidence, Finding, Report, Verdict};

fn tool(id: &str, license: &str, class: &str) -> Tool {
    Tool {
        id: id.into(),
        registry: "crates_io".into(),
        url: format!("https://crates.io/crates/{id}"),
        license: license.into(),
        class: class.into(),
    }
}

struct Fake(Result<Reuse, String>);

impl ExistingTools for Fake {
    fn find(&self, _: &str) -> Result<Reuse> {
        self.0.clone().map_err(|e| anyhow::anyhow!(e))
    }
}

fn run(reuse: Reuse, force: bool) -> (Result<()>, String) {
    let mut err = Vec::new();
    let got = check("serialize json", force, &Fake(Ok(reuse)), &mut err);
    (got, String::from_utf8_lossy(&err).into_owned())
}

#[test]
fn nothing_found_creates_without_a_word() {
    let (got, err) = run(Reuse::default(), false);
    assert!(got.is_ok());
    assert!(err.is_empty(), "{err}");
}

#[test]
fn a_reusable_tool_stops_the_creation_and_is_named_with_its_licence() {
    let reuse = Reuse { reusable: vec![tool("serde", "MIT OR Apache-2.0", "permissive")], ..Reuse::default() };
    let (got, err) = run(reuse, false);
    let msg = got.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("--force"), "{msg}");
    for text in [&msg, &err] {
        assert!(text.contains("serde") && text.contains("https://crates.io/crates/serde"), "{text}");
        assert!(text.contains("MIT OR Apache-2.0"), "{text}");
    }
}

#[test]
fn force_creates_anyway_and_still_says_what_exists() {
    let reuse = Reuse { reusable: vec![tool("serde", "MIT", "permissive")], ..Reuse::default() };
    let (got, err) = run(reuse, true);
    assert!(got.is_ok());
    assert!(err.contains("warn") && err.contains("serde"), "{err}");
}

#[test]
fn a_tool_that_cannot_be_reused_does_not_stop_but_is_reported_with_the_class() {
    let reuse = Reuse {
        blocked: vec![tool("viral", "AGPL-3.0-only", "network-copyleft"), tool("mystery", "none declared", "unknown")],
        ..Reuse::default()
    };
    let (got, err) = run(reuse, false);
    assert!(got.is_ok());
    assert!(err.contains("viral") && err.contains("network-copyleft"), "{err}");
    assert!(err.contains("mystery") && err.contains("unknown"), "{err}");
    assert!(err.contains("not reusable"), "{err}");
}

#[test]
fn unreadable_candidates_are_warned_about_not_swallowed() {
    let reuse = Reuse { unreadable: vec!["foo (npm): login wall".into()], ..Reuse::default() };
    let (got, err) = run(reuse, false);
    assert!(got.is_ok());
    assert!(err.contains("warn") && err.contains("login wall"), "{err}");
}

#[test]
fn a_failing_lookup_aborts_even_with_force() {
    for force in [false, true] {
        let mut err = Vec::new();
        let got = check("x", force, &Fake(Err("indexer: unreachable".into())), &mut err);
        assert!(got.err().is_some_and(|e| e.to_string().contains("indexer: unreachable")), "force={force}");
    }
}

fn finding(id: &str, license: Option<&str>) -> Finding {
    Finding {
        candidate: Candidate {
            id: id.into(),
            registry: "crates_io".into(),
            url: format!("https://crates.io/crates/{id}"),
            docs_url: None,
            description: String::new(),
            license: license.map(str::to_owned),
        },
        term_hits: 1,
        evidence: Evidence { location: "l".into(), snippet: "s".into(), confidence: 0.9 },
    }
}

#[test]
fn an_outcome_is_split_into_reusable_blocked_and_unreadable() {
    let report = Report {
        verdict: Verdict::Exists,
        findings: vec![finding("free", Some("MIT")), finding("viral", Some("AGPL-3.0-only")), finding("bare", None)],
        failures: vec![(
            finding("hidden", None).candidate,
            actit::error::Error::Kag("login wall".into()),
        )],
    };
    let licenses = report
        .findings
        .iter()
        .map(|f| licence(f.candidate.license.as_deref(), &Table::builtin(), &default_allow()))
        .collect();
    let reuse = reuse_of(&Outcome { report, licenses });
    assert_eq!(reuse.reusable, [tool("free", "MIT", "permissive")]);
    assert_eq!(
        reuse.blocked,
        [tool("viral", "AGPL-3.0-only", "network-copyleft"), tool("bare", "none declared", "unknown")]
    );
    assert_eq!(reuse.unreadable.len(), 1);
    assert!(reuse.unreadable[0].contains("hidden") && reuse.unreadable[0].contains("login wall"));
}

fn outcome(verdict: Verdict) -> Outcome {
    let report = Report { verdict, findings: vec![finding("argos", Some("MIT"))], failures: vec![] };
    let licenses = report
        .findings
        .iter()
        .map(|f| licence(f.candidate.license.as_deref(), &Table::builtin(), &default_allow()))
        .collect();
    Outcome { report, licenses }
}

#[test]
fn a_candidate_no_documentation_confirms_never_stops_the_creation() {
    let reuse = reuse_of(&outcome(Verdict::Unconfirmed));
    assert!(reuse.reusable.is_empty(), "{reuse:?}");
    assert_eq!(reuse.unconfirmed, [tool("argos", "MIT", "permissive")]);
    let (got, err) = run(reuse, false);
    assert!(got.is_ok());
    assert!(err.contains("argos") && err.contains("not confirmed"), "{err}");
}

#[test]
fn a_confirmed_candidate_still_stops_the_creation() {
    let reuse = reuse_of(&outcome(Verdict::Exists));
    assert_eq!(reuse.reusable, [tool("argos", "MIT", "permissive")]);
    assert!(reuse.unconfirmed.is_empty());
}
