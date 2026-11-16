//! A failing manifest stops itself and what depends on it, unless `continue_on_error` lets it go on

use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[test]
fn a_failure_stops_only_the_manifests_that_depend_on_it() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = Loaded {
        manifests: HashMap::from([
            manifest("app", &["broken"], &dir.path().join("app")),
            manifest("other", &[], &dir.path().join("other")),
        ]),
        errors: vec![broken("broken")],
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

    assert_eq!(vec!["app", "broken"], report.failed);
    assert!(!dir.path().join("app").exists());
    assert!(dir.path().join("other").is_dir());
    assert_eq!(Err(RunError::Failed(2)), report.into_result());
}

#[cfg(unix)]
#[test]
fn continue_on_error_goes_on_with_the_next_action() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = format!(
        "actions:\n  - action: command.run\n    command: 'false'\n    continue_on_error: true\n  - action: directory.create\n    path: '{}'\n",
        dir.path().join("after").display()
    );
    let loaded = Loaded {
        manifests: HashMap::from([
            manifest_from("tolerant", &yaml),
            manifest("dependent", &["tolerant"], &dir.path().join("dependent")),
        ]),
        errors: vec![],
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("after").is_dir());
    assert!(dir.path().join("dependent").is_dir());
}

#[cfg(unix)]
#[test]
fn without_continue_on_error_a_failure_stops_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = format!(
        "actions:\n  - action: command.run\n    command: 'false'\n  - action: directory.create\n    path: '{}'\n",
        dir.path().join("after").display()
    );
    let loaded = Loaded {
        manifests: HashMap::from([manifest_from("strict", &yaml)]),
        errors: vec![],
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

    assert_eq!(vec!["strict"], report.failed);
    assert!(!dir.path().join("after").exists());
}
