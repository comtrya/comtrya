//! `register` (manifest version 2): later actions read what a command printed

use super::*;
use crate::contexts::Contexts;
use crate::manifests::load_all;
use crate::utilities::error_chain;
use pretty_assertions::assert_eq;
use std::fs;
use std::sync::Mutex;

/// Loads `manifests` (name and contents, version 2 added) from a directory and runs them
/// with `dry_run`. Returns the report and what the log would say about skipped or failed
/// actions
fn run_v2(dir: &Path, manifests: &[(&str, &str)], dry_run: bool) -> (Report, Vec<String>) {
    let manifest_dir = dir.join("manifests");
    fs::create_dir_all(&manifest_dir).unwrap();
    for (name, contents) in manifests {
        fs::write(
            manifest_dir.join(format!("{name}.yaml")),
            format!("version: 2\n{contents}"),
        )
        .unwrap();
    }
    let loaded = load_all(manifest_dir.clone(), &Contexts::default());
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);

    let said = Mutex::new(vec![]);
    let observer = |event: &Event| {
        let line = match event {
            Event::ActionPlanFailed(error) => error_chain(*error),
            Event::ActionSkipped(ActionSkip::ReadsRegistered) => String::from("skipped"),
            _ => return,
        };
        said.lock().unwrap().push(line);
    };
    let options = Options {
        dry_run,
        observer: &observer,
        ..Default::default()
    };

    let report = run(loaded, &manifest_dir, PlanCx::empty(), &options);
    (report, said.into_inner().unwrap())
}

/// A `command.run` of `sh -c script` that registers `name`
fn command(name: &str, script: &str) -> String {
    format!(
        "  - action: command.run\n    command: sh\n    args: [-c, \"{script}\"]\n    register: {name}\n"
    )
}

/// A `directory.create` of `path`, with more `fields`
fn create(path: &str, fields: &str) -> String {
    format!("  - action: directory.create\n    path: \"{path}\"\n{fields}")
}

#[test]
fn a_later_action_reads_what_a_command_printed() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}{}",
        command("greeting", "echo hi; echo oops >&2"),
        create(
            &format!(
                "{base}/{{{{ registered.greeting.stdout }}}}-{{{{ registered.greeting.stderr }}}}"
            ),
            ""
        ),
    );

    let (report, said) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(Report::default(), report);
    assert_eq!(Vec::<String>::new(), said);
    assert!(dir.path().join("hi-oops").is_dir());
}

#[test]
fn where_reads_the_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}{}{}",
        command("check", "true"),
        create(
            &format!("{base}/ok"),
            "    where: registered.check.exit_code == 0\n"
        ),
        create(
            &format!("{base}/not"),
            "    where: registered.check.exit_code != 0\n"
        ),
    );

    let (report, _) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("ok").is_dir());
    assert!(!dir.path().join("not").exists());
}

#[test]
fn a_failed_command_with_continue_on_error_registers_its_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}    continue_on_error: true\n{}",
        command("probe", "echo out; exit 3"),
        create(
            &format!(
                "{base}/{{{{ registered.probe.stdout }}}}-{{{{ registered.probe.exit_code }}}}"
            ),
            ""
        ),
    );

    let (report, _) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("out-3").is_dir());
}

#[test]
fn a_dependent_manifest_reads_what_its_dependency_registered() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let first = format!("actions:\n{}", command("name", "echo shared"));
    let second = format!(
        "depends: [first]\nactions:\n{}",
        create(&format!("{base}/{{{{ registered.name.stdout }}}}"), "")
    );

    let (report, _) = run_v2(dir.path(), &[("first", &first), ("second", &second)], false);

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("shared").is_dir());
}

#[test]
fn reading_a_name_nothing_registered_fails_the_action() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}",
        create(&format!("{base}/{{{{ registered.nope.stdout }}}}"), "")
    );

    let (report, said) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(vec![String::from("a")], report.failed);
    assert_eq!(1, said.len());
    assert!(
        said[0].starts_with("`registered.nope` isn't set"),
        "{said:?}"
    );
}

#[test]
fn a_where_that_reads_a_name_nothing_registered_fails_the_action() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}",
        create(
            &format!("{base}/x"),
            "    where: registered.nope.exit_code == 0\n"
        )
    );

    let (report, said) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(vec![String::from("a")], report.failed);
    assert!(
        said[0].starts_with("`registered.nope` isn't set"),
        "{said:?}"
    );
}

#[test]
fn a_dry_run_skips_actions_that_read_registered_values() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}{}{}",
        command("greeting", "echo hi"),
        create(&format!("{base}/{{{{ registered.greeting.stdout }}}}"), ""),
        create(
            &format!("{base}/x"),
            "    where: registered.greeting.exit_code == 0\n"
        ),
    );

    let (report, said) = run_v2(dir.path(), &[("a", &actions)], true);

    assert_eq!(Report::default(), report);
    assert_eq!(vec!["skipped", "skipped"], said);
    assert!(!dir.path().join("hi").exists());
}

#[test]
fn a_variant_reads_registered_values_it_takes_from_its_action() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().display();
    let actions = format!(
        "actions:\n{}{}",
        command("greeting", "echo hi"),
        create(
            &format!("{base}/{{{{ registered.greeting.stdout }}}}"),
            "    variants:\n      - where: \"true\"\n        privileged: false\n"
        ),
    );

    let (report, said) = run_v2(dir.path(), &[("a", &actions)], false);

    assert_eq!(Report::default(), report);
    assert_eq!(Vec::<String>::new(), said);
    assert!(dir.path().join("hi").is_dir());
}
