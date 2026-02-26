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
