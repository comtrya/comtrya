//! `comtrya __helper`, the privileged helper, run without privileges

use assert_cmd::Command;
use comtrya_lib::atoms::{file::Create, Atom};

#[test]
fn the_helper_runs_the_atoms_it_is_sent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("made-by-the-helper");
    let request = serde_json::to_string(&Atom::from(Create { path: file.clone() })).unwrap();

    let output = Command::cargo_bin("comtrya")
        .unwrap()
        .arg("__helper")
        .write_stdin(format!("{request}\n"))
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        "{\"Ok\":{\"stdout\":\"\",\"stderr\":\"\"}}\n",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(file.is_file());
}

#[test]
fn the_helper_is_hidden() {
    let help = Command::cargo_bin("comtrya")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();

    assert!(!String::from_utf8_lossy(&help.stdout).contains("__helper"));
}

/// A `sudo` that counts how often it's started, and runs the command as it is
#[cfg(unix)]
fn fake_sudo(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let bin = dir.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let sudo = bin.join("sudo");
    let script = format!(
        "#!/bin/sh\necho started >> '{}'\nexec \"$@\"\n",
        dir.join("sudo.log").display()
    );
    std::fs::write(&sudo, script).unwrap();
    std::fs::set_permissions(&sudo, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[cfg(unix)]
#[test]
fn privileged_actions_share_one_helper() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_sudo(dir.path());
    let manifests = dir.path().join("manifests");
    std::fs::create_dir(&manifests).unwrap();
    let out = dir.path().join("out");
    std::fs::write(
        manifests.join("root.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: '{0}/a'\n    privileged: true\n  - action: directory.create\n    path: '{0}/b'\n    sudo: true\n",
            out.display()
        ),
    )
    .unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());

    Command::cargo_bin("comtrya")
        .unwrap()
        .env("PATH", path)
        .args(["--no-color", "-d"])
        .arg(&manifests)
        .arg("apply")
        .assert()
        .success();

    assert!(out.join("a").is_dir());
    assert!(out.join("b").is_dir());
    let starts = std::fs::read_to_string(dir.path().join("sudo.log")).unwrap();
    assert_eq!(1, starts.lines().count(), "{starts}");
}
