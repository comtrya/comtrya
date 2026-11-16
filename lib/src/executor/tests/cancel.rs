//! Ctrl-C: a cancel stops the run before the next step

use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[test]
fn a_cancel_stops_before_the_next_step() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = Loaded {
        manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
        errors: vec![],
    };
    let cancel = AtomicBool::new(false);
    let events = std::sync::Mutex::new(vec![]);
    // Ctrl-C while the action plans, right before its step would run
    let record = |event: &Event| {
        if let Event::ActionStarted { .. } = event {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        events.lock().unwrap().push(name(event));
    };
    let options = Options {
        observer: &record,
        cancel: Some(&cancel),
        ..Default::default()
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &options);

    assert!(report.cancelled);
    assert!(!dir.path().join("app").exists());
    assert!(events.lock().unwrap().contains(&String::from("cancelled")));
    assert_eq!(Err(RunError::Cancelled), report.into_result());
}

#[test]
fn continue_on_error_doesnt_go_on_after_a_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = format!(
        "actions:\n  - action: directory.create\n    path: '{0}/a'\n    continue_on_error: true\n  - action: directory.create\n    path: '{0}/b'\n",
        dir.path().display()
    );
    let loaded = Loaded {
        manifests: HashMap::from([manifest_from("tolerant", &yaml)]),
        errors: vec![],
    };
    let cancel = AtomicBool::new(false);
    // Ctrl-C while the first action plans: its step and everything after it don't run
    let record = |event: &Event| {
        if let Event::ActionStarted { .. } = event {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    };
    let options = Options {
        observer: &record,
        cancel: Some(&cancel),
        ..Default::default()
    };

    let report = run(loaded, dir.path(), PlanCx::empty(), &options);

    assert!(report.cancelled);
    assert!(!dir.path().join("a").exists());
    assert!(!dir.path().join("b").exists());
}
