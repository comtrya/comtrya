//! Black-box tests for `comtrya manifests upgrade`, through the binary

use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

/// A version 1 manifest with an old action name and a field alias
const VERSION_1: &str = "actions:\n  - action: cmd.run\n    command: echo\n    sudo: true\n";

/// [`VERSION_1`] upgraded
const VERSION_2: &str =
    "version: 2\nactions:\n- action: command.run\n  command: echo\n  privileged: true\n";

/// `comtrya manifests upgrade` with `args`, isolated from the user's own config
fn upgrade(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("comtrya"));
    cmd.current_dir(dir)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .args(["--no-color", "manifests", "upgrade"])
        .args(args);
    cmd
}

#[test]
fn prints_the_upgraded_manifest_and_leaves_the_file_alone() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("m.yaml"), VERSION_1).unwrap();

    upgrade(dir.path(), &["m.yaml"])
        .assert()
        .success()
        .stdout(VERSION_2);

    assert_eq!(
        VERSION_1,
        std::fs::read_to_string(dir.path().join("m.yaml")).unwrap()
    );
}

#[test]
fn write_replaces_the_file() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("m.yaml"), VERSION_1).unwrap();

    upgrade(dir.path(), &["--write", "m.yaml"])
        .assert()
        .success()
        .stdout("");

    assert_eq!(
        VERSION_2,
        std::fs::read_to_string(dir.path().join("m.yaml")).unwrap()
    );
}

#[test]
fn a_manifest_that_cant_be_upgraded_fails_and_stays_as_it_is() {
    let dir = TempDir::new().unwrap();
    let text = "actions:\n{% if true %}\n  - action: command.run\n    command: echo\n{% endif %}\n";
    std::fs::write(dir.path().join("m.yaml"), text).unwrap();

    upgrade(dir.path(), &["--write", "m.yaml"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("m.yaml isn't upgraded"));

    assert_eq!(
        text,
        std::fs::read_to_string(dir.path().join("m.yaml")).unwrap()
    );
}
