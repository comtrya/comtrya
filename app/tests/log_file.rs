//! `--log-file`: the log goes to a file as well

use assert_cmd::Command;

#[test]
fn the_log_is_written_to_the_file_too() {
    let dir = tempfile::tempdir().unwrap();
    let manifests = dir.path().join("manifests");
    std::fs::create_dir(&manifests).unwrap();
    let created = dir.path().join("created");
    std::fs::write(
        manifests.join("logged.yaml"),
        format!(
            "actions:\n  - action: directory.create\n    path: '{}'\n",
            created.display()
        ),
    )
    .unwrap();
    let log = dir.path().join("comtrya.log");
    std::fs::write(&log, "from before\n").unwrap();

    let output = Command::cargo_bin("comtrya")
        .unwrap()
        .arg("--log-file")
        .arg(&log)
        .arg("-d")
        .arg(&manifests)
        .arg("apply")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let written = std::fs::read_to_string(&log).unwrap();
    assert!(written.starts_with("from before\n"), "{written}");
    assert!(written.contains("manifest=\"logged\""), "{written}");
    assert!(written.contains("Completed"), "{written}");
    // Plain text, without the colors of the terminal
    assert!(!written.contains('\u{1b}'), "{written}");
    assert!(stdout.contains("Completed"), "{stdout}");
}

#[test]
fn a_log_file_that_cant_be_opened_is_an_error() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::cargo_bin("comtrya")
        .unwrap()
        .arg("--log-file")
        .arg(dir.path().join("no-such-dir").join("comtrya.log"))
        .arg("version")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("comtrya.log"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
