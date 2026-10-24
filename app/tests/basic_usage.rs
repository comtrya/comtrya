use tempfile::TempDir;
use utils::*;

mod utils;

#[test]
fn prints_help() {
    run("-h")
        .success()
        .stdout(predicates::str::contains("comtrya"));
}

#[test]
fn dry_run_doesnt_error() {
    let t = TempDir::new().expect("could not create tempdir");
    let path = t.keep();
    dir(
        "directory",
        vec![dir(
            "copy",
            vec![
                dir(
                    "files",
                    vec![dir(
                        "mydir",
                        vec![
                            f("file-a", "some content a"),
                            f("file-b", "some other thing"),
                        ],
                    )],
                ),
                f(
                    "main.yaml",
                    r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
                ),
                f(
                    "where_condition.yaml",
                    r#"
where: non.existing.variable == true

actions:
  - action: command.run
    command: echo
    args:
      - hello, world!
                    "#,
                ),
            ],
        )],
    )
    .create_in(&path)
    .expect("should have create test directories");

    let assert = cd(path).run("--no-color -d ./directory apply -m copy --dry-run");

    assert.success();
}

#[test]
fn apply_without_config_uses_current_directory() {
    let t = TempDir::new().expect("could not create tempdir");
    f(
        "hello.yaml",
        r#"
actions:
  - action: command.run
    command: echo
    args:
      - hello
"#,
    )
    .create_in(t.path())
    .expect("should have created manifest");

    // Point the platform config dir at the empty tempdir, so no Comtrya.yaml is found
    assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("comtrya"))
        .current_dir(t.path())
        .env("HOME", t.path())
        .env("XDG_CONFIG_HOME", t.path())
        .args(["--no-color", "apply", "--dry-run"])
        .assert()
        .success();
}

#[test]
fn contexts_shows_git_manifest_cache() {
    let t = TempDir::new().expect("could not create tempdir");

    cd(t.path().to_path_buf())
        .run("--no-color contexts")
        .success()
        .stdout(predicates::str::contains("git manifest cache"))
        .stdout(predicates::str::contains("manifests"));
}
