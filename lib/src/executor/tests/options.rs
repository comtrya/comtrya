//! What `--dry-run` and choosing manifests change about a run

use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[test]
fn a_dry_run_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = Loaded {
        manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
        errors: vec![],
    };
    let options = Options {
        dry_run: true,
        ..Default::default()
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &options);

    assert_eq!(Report::default(), report);
    assert!(!dir.path().join("app").exists());
}

#[test]
fn a_broken_manifest_that_isnt_selected_doesnt_fail_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = Loaded {
        manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
        errors: vec![broken("broken")],
    };
    let selected = [String::from("app")];
    let options = Options {
        manifests: &selected,
        ..Default::default()
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &options);

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("app").is_dir());
}
