//! The events a run tells its observer, in the order things happen

use super::*;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

#[test]
fn events_tell_what_happens_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = Loaded {
        manifests: HashMap::from([manifest("app", &["broken"], &dir.path().join("app"))]),
        errors: vec![broken("broken")],
    };
    let loaded_ok = Loaded {
        manifests: HashMap::from([manifest("ok", &[], &dir.path().join("ok"))]),
        errors: vec![],
    };
    let events = std::sync::Mutex::new(vec![]);
    let record = |event: &Event| events.lock().unwrap().push(name(event));
    let options = Options {
        observer: &record,
        ..Default::default()
    };

    run(loaded, dir.path(), PlanCx::empty(), &options);
    run(loaded_ok, dir.path(), PlanCx::empty(), &options);

    assert_eq!(
        vec![
            "broken",
            "start app",
            "skip: dependency",
            "start ok",
            "action directory.create",
            "step",
            "action done: true",
            "done, failed: false",
        ],
        events.into_inner().unwrap()
    );
}

#[cfg(unix)]
#[test]
fn lines_a_program_prints_are_events() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = "actions:\n  - action: command.run\n    command: sh\n    args: [-c, 'echo one; echo two >&2']\n";
    let manifest = Manifest {
        name: Some(String::from("echo")),
        ..serde_yaml_ng::from_str(yaml).unwrap()
    };
    let loaded = Loaded {
        manifests: HashMap::from([(String::from("echo"), manifest)]),
        errors: vec![],
    };
    let lines = std::sync::Mutex::new(vec![]);
    let record = |event: &Event| {
        if let Event::StepOutput { stream, line } = event {
            lines.lock().unwrap().push(format!("{stream:?} {line}"));
        }
    };
    let options = Options {
        observer: &record,
        ..Default::default()
    };

    run(loaded, dir.path(), PlanCx::empty(), &options);

    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    assert_eq!(vec!["Stderr two", "Stdout one"], lines);
}
